// Copyright 2023 RisingWave Labs
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::HashMap;
use std::sync::LazyLock;

use itertools::Itertools;
use maplit::{convert_args, hashmap};
use pgwire::pg_response::{PgResponse, StatementType};
use risingwave_common::catalog::{
    is_column_ids_dedup, ColumnCatalog, ColumnDesc, TableId, DEFAULT_KEY_COLUMN_NAME,
};
use risingwave_common::error::ErrorCode::{self, InvalidInputSyntax, ProtocolError};
use risingwave_common::error::{Result, RwError};
use risingwave_common::types::DataType;
use risingwave_connector::parser::{
    AvroParserConfig, DebeziumAvroParserConfig, ParserProperties, ProtobufParserConfig,
};
use risingwave_connector::source::cdc::{
    CITUS_CDC_CONNECTOR, MYSQL_CDC_CONNECTOR, POSTGRES_CDC_CONNECTOR,
};
use risingwave_connector::source::datagen::DATAGEN_CONNECTOR;
use risingwave_connector::source::filesystem::S3_CONNECTOR;
use risingwave_connector::source::nexmark::source::{get_event_data_types_with_names, EventType};
use risingwave_connector::source::{
    SourceFormat, GOOGLE_PUBSUB_CONNECTOR, KAFKA_CONNECTOR, KINESIS_CONNECTOR, NEXMARK_CONNECTOR,
    PULSAR_CONNECTOR,
};
use risingwave_pb::catalog::{PbSource, StreamSourceInfo, WatermarkDesc};
use risingwave_pb::plan_common::RowFormatType;
use risingwave_sqlparser::ast::{
    self, AvroSchema, ColumnDef, ColumnOption, CompatibleSourceSchema, CreateSourceStatement,
    DebeziumAvroSchema, Encode, ProtobufSchema, SourceSchema, SourceWatermark,
};

use super::RwPgResponse;
use crate::binder::Binder;
use crate::catalog::ColumnId;
use crate::expr::Expr;
use crate::handler::create_table::{
    bind_pk_names, bind_pk_on_relation, bind_sql_column_constraints, bind_sql_columns,
    ensure_table_constraints_supported, ColumnIdGenerator,
};
use crate::handler::util::{get_connector, is_kafka_connector};
use crate::handler::HandlerArgs;
use crate::optimizer::plan_node::KAFKA_TIMESTAMP_COLUMN_NAME;
use crate::session::SessionImpl;
use crate::utils::resolve_connection_in_with_option;
use crate::{bind_data_type, WithOptions};

pub(crate) const UPSTREAM_SOURCE_KEY: &str = "connector";
pub(crate) const CONNECTION_NAME_KEY: &str = "connection.name";

/// Map an Avro schema to a relational schema.
async fn extract_avro_table_schema(
    schema: &AvroSchema,
    with_properties: &HashMap<String, String>,
) -> Result<Vec<ColumnCatalog>> {
    let info = StreamSourceInfo {
        row_schema_location: schema.row_schema_location.0.clone(),
        use_schema_registry: schema.use_schema_registry,
        ..Default::default()
    };
    let parser_config = ParserProperties::new(SourceFormat::Avro, with_properties, &info)?;
    let conf = AvroParserConfig::new(parser_config.encoding_config).await?;
    let vec_column_desc = conf.map_to_columns()?;
    Ok(vec_column_desc
        .into_iter()
        .map(|col| ColumnCatalog {
            column_desc: col.into(),
            is_hidden: false,
        })
        .collect_vec())
}

/// Map an Avro schema to a relational schema. And extract primary key columns.
async fn extract_upsert_avro_table_schema(
    schema: &AvroSchema,
    with_properties: &HashMap<String, String>,
) -> Result<(Vec<ColumnCatalog>, Vec<String>)> {
    let info = StreamSourceInfo {
        row_schema_location: schema.row_schema_location.0.clone(),
        use_schema_registry: schema.use_schema_registry,
        ..Default::default()
    };
    let parser_config = ParserProperties::new(SourceFormat::UpsertAvro, with_properties, &info)?;
    let conf = AvroParserConfig::new(parser_config.encoding_config).await?;
    let vec_column_desc = conf.map_to_columns()?;
    let mut vec_column_catalog = vec_column_desc
        .clone()
        .into_iter()
        .map(|col| ColumnCatalog {
            column_desc: col.into(),
            is_hidden: false,
        })
        .collect_vec();

    // For upsert avro, if we can't extract pk from schema, use message key as primary key
    let pks = if let Ok(pk_desc) = conf.extract_pks() {
        pk_desc
            .into_iter()
            .map(|desc| {
                vec_column_desc
                    .iter()
                    .find(|x| x.name == desc.name)
                    .ok_or_else(|| {
                        RwError::from(ErrorCode::InternalError(format!(
                            "Can not found primary key column {} in value schema",
                            desc.name
                        )))
                    })
            })
            .map_ok(|desc| desc.name.clone())
            .collect::<Result<Vec<_>>>()?
    } else {
        let kafka_key_column = ColumnCatalog {
            column_desc: ColumnDesc {
                data_type: DataType::Bytea,
                column_id: (vec_column_catalog.len() as i32).into(),
                name: DEFAULT_KEY_COLUMN_NAME.to_string(),
                field_descs: vec![],
                type_name: "".to_string(),
                generated_or_default_column: None,
            },
            is_hidden: true,
        };
        vec_column_catalog.push(kafka_key_column);
        vec![DEFAULT_KEY_COLUMN_NAME.into()]
    };
    Ok((vec_column_catalog, pks))
}

async fn extract_debezium_avro_table_pk_columns(
    schema: &DebeziumAvroSchema,
    with_properties: &HashMap<String, String>,
) -> Result<Vec<String>> {
    let info = StreamSourceInfo {
        row_schema_location: schema.row_schema_location.0.clone(),
        ..Default::default()
    };
    let parser_config = ParserProperties::new(SourceFormat::DebeziumAvro, with_properties, &info)?;
    let conf = DebeziumAvroParserConfig::new(parser_config.encoding_config).await?;
    Ok(conf.extract_pks()?.drain(..).map(|c| c.name).collect())
}

// Map an Avro schema to a relational schema and return the pk_column_ids.
async fn extract_debezium_avro_table_schema(
    schema: &DebeziumAvroSchema,
    with_properties: &HashMap<String, String>,
) -> Result<Vec<ColumnCatalog>> {
    let info = StreamSourceInfo {
        row_schema_location: schema.row_schema_location.0.clone(),
        ..Default::default()
    };
    let parser_config = ParserProperties::new(SourceFormat::DebeziumAvro, with_properties, &info)?;
    let conf = DebeziumAvroParserConfig::new(parser_config.encoding_config).await?;
    let vec_column_desc = conf.map_to_columns()?;
    let column_catalog = vec_column_desc
        .into_iter()
        .map(|col| ColumnCatalog {
            column_desc: col.into(),
            is_hidden: false,
        })
        .collect_vec();
    Ok(column_catalog)
}

/// Map a protobuf schema to a relational schema.
async fn extract_protobuf_table_schema(
    schema: &ProtobufSchema,
    with_properties: HashMap<String, String>,
) -> Result<Vec<ColumnCatalog>> {
    let info = StreamSourceInfo {
        proto_message_name: schema.message_name.0.clone(),
        row_schema_location: schema.row_schema_location.0.clone(),
        use_schema_registry: schema.use_schema_registry,
        ..Default::default()
    };
    let parser_config = ParserProperties::new(SourceFormat::Protobuf, &with_properties, &info)?;
    let conf = ProtobufParserConfig::new(parser_config.encoding_config).await?;

    let column_descs = conf.map_to_columns()?;

    Ok(column_descs
        .into_iter()
        .map(|col| ColumnCatalog {
            column_desc: col.into(),
            is_hidden: false,
        })
        .collect_vec())
}

fn non_generated_sql_columns(columns: &[ColumnDef]) -> Vec<ColumnDef> {
    columns
        .iter()
        .filter(|c| {
            c.options
                .iter()
                .all(|option| !matches!(option.option, ColumnOption::GeneratedColumns(_)))
        })
        .cloned()
        .collect()
}

/// resolve the schema of the source from external schema file, return the relation's columns. see <https://www.risingwave.dev/docs/current/sql-create-source> for more information.
/// return `(columns, pk_names, source info)`
pub(crate) async fn try_bind_columns_from_source(
    source_schema: &SourceSchema,
    sql_defined_pk_names: Vec<String>,
    sql_defined_columns: &[ColumnDef],
    with_properties: &HashMap<String, String>,
) -> Result<(Option<Vec<ColumnCatalog>>, Vec<String>, StreamSourceInfo)> {
    let sql_defined_pk = !sql_defined_pk_names.is_empty();
    let sql_defined_schema = !sql_defined_columns.is_empty();
    let is_kafka: bool = is_kafka_connector(with_properties);

    Ok(match source_schema {
        SourceSchema::Protobuf(protobuf_schema) => {
            if sql_defined_schema {
                return Err(RwError::from(ProtocolError(
                    "User-defined schema is not allowed with FORMAT PLAIN ENCODE PROTOBUF. Please refer to https://www.risingwave.dev/docs/current/sql-create-source/#protobuf for more information.".to_string())));
            };
            (
                Some(
                    extract_protobuf_table_schema(protobuf_schema, with_properties.clone()).await?,
                ),
                sql_defined_pk_names,
                StreamSourceInfo {
                    row_format: RowFormatType::Protobuf as i32,
                    row_schema_location: protobuf_schema.row_schema_location.0.clone(),
                    use_schema_registry: protobuf_schema.use_schema_registry,
                    proto_message_name: protobuf_schema.message_name.0.clone(),
                    ..Default::default()
                },
            )
        }
        SourceSchema::Avro(avro_schema) => {
            if sql_defined_schema {
                return Err(RwError::from(ProtocolError(
                    "User-defined schema is not allowed with FORMAT PLAIN ENCODE AVRO. Please refer to https://www.risingwave.dev/docs/current/sql-create-source/#avro for more information.".to_string())));
            }
            (
                Some(extract_avro_table_schema(avro_schema, with_properties).await?),
                sql_defined_pk_names,
                StreamSourceInfo {
                    row_format: RowFormatType::Avro as i32,
                    row_schema_location: avro_schema.row_schema_location.0.clone(),
                    use_schema_registry: avro_schema.use_schema_registry,
                    proto_message_name: "".to_owned(),
                    ..Default::default()
                },
            )
        }
        SourceSchema::UpsertAvro(avro_schema) => {
            if sql_defined_schema {
                return Err(RwError::from(ProtocolError(
                    "User-defined schema is not allowed with FORMAT UPSERT ENCODE AVRO. Please refer to https://www.risingwave.dev/docs/current/sql-create-source/#upsert-avro for more information.".to_string())));
            }

            if sql_defined_pk {
                if sql_defined_pk_names.len() != 1 {
                    return Err(RwError::from(ProtocolError(
                        "upsert avro supports only one primary key column.".to_string(),
                    )));
                }
                let columns = extract_avro_table_schema(avro_schema, with_properties).await?;

                let upsert_avro_primary_key = sql_defined_pk_names[0].clone();
                (
                    Some(columns),
                    sql_defined_pk_names,
                    StreamSourceInfo {
                        row_format: RowFormatType::UpsertAvro as i32,
                        row_schema_location: avro_schema.row_schema_location.0.clone(),
                        use_schema_registry: avro_schema.use_schema_registry,
                        upsert_avro_primary_key,
                        ..Default::default()
                    },
                )
            } else {
                let (columns, pk_from_avro) =
                    extract_upsert_avro_table_schema(avro_schema, with_properties).await?;
                (
                    Some(columns),
                    pk_from_avro,
                    StreamSourceInfo {
                        row_format: RowFormatType::UpsertAvro as i32,
                        row_schema_location: avro_schema.row_schema_location.0.clone(),
                        use_schema_registry: avro_schema.use_schema_registry,
                        ..Default::default()
                    },
                )
            }
        }
        SourceSchema::DebeziumAvro(avro_schema) => {
            if sql_defined_schema {
                return Err(RwError::from(ProtocolError(
                    "User-defined schema is not allowed with row format debezium avro.".to_string(),
                )));
            }
            let full_columns =
                extract_debezium_avro_table_schema(avro_schema, with_properties).await?;

            let pk_names = if sql_defined_pk {
                sql_defined_pk_names
            } else {
                let pk_names =
                    extract_debezium_avro_table_pk_columns(avro_schema, with_properties).await?;
                // extract pk(s) from schema registry
                for pk_name in &pk_names {
                    full_columns
                        .iter()
                        .find(|c: &&ColumnCatalog| c.name().eq(pk_name))
                        .ok_or_else(|| {
                            RwError::from(ProtocolError(format!(
                                "avro's key column {} not exists in avro's row schema",
                                pk_name
                            )))
                        })?;
                }
                pk_names
            };
            (
                Some(full_columns),
                pk_names,
                StreamSourceInfo {
                    row_format: RowFormatType::DebeziumAvro as i32,
                    row_schema_location: avro_schema.row_schema_location.0.clone(),
                    ..Default::default()
                },
            )
        }

        SourceSchema::DebeziumJson => {
            if !sql_defined_pk {
                return Err(RwError::from(ProtocolError(
                    "Primary key must be specified when creating source with row format debezium."
                        .to_string(),
                )));
            }
            (
                None,
                sql_defined_pk_names,
                StreamSourceInfo {
                    row_format: RowFormatType::DebeziumJson as i32,
                    ..Default::default()
                },
            )
        }
        SourceSchema::UpsertJson => {
            if !sql_defined_pk {
                return Err(RwError::from(ProtocolError(
                    "Primary key must be specified when creating source with FORMAT UPSERT ENCODE JSON."
                        .to_string(),
                )));
            }
            (
                None,
                sql_defined_pk_names,
                StreamSourceInfo {
                    row_format: RowFormatType::UpsertJson as i32,
                    ..Default::default()
                },
            )
        }
        SourceSchema::Maxwell => {
            if !sql_defined_pk {
                return Err(RwError::from(ProtocolError(
                    "Primary key must be specified when creating source with FORMAT MAXWELL ENCODE JSON."
                        .to_string(),
                )));
            }
            (
                None,
                sql_defined_pk_names,
                StreamSourceInfo {
                    row_format: RowFormatType::Maxwell as i32,
                    ..Default::default()
                },
            )
        }
        SourceSchema::CanalJson => {
            if !sql_defined_pk {
                return Err(RwError::from(ProtocolError(
                    "Primary key must be specified when creating source with row format cannal_json."
                        .to_string(),
                )));
            }
            (
                None,
                sql_defined_pk_names,
                StreamSourceInfo {
                    row_format: RowFormatType::CanalJson as i32,
                    ..Default::default()
                },
            )
        }
        SourceSchema::Csv(csv_info) => {
            if is_kafka && csv_info.has_header {
                return Err(RwError::from(ProtocolError(
                    "CSV HEADER is not supported when creating table with Kafka connector"
                        .to_owned(),
                )));
            }
            (
                None,
                sql_defined_pk_names,
                StreamSourceInfo {
                    row_format: RowFormatType::Csv as i32,
                    csv_delimiter: csv_info.delimiter as i32,
                    csv_has_header: csv_info.has_header,
                    ..Default::default()
                },
            )
        }
        SourceSchema::Json => (
            None,
            sql_defined_pk_names,
            StreamSourceInfo {
                row_format: RowFormatType::Json as i32,
                ..Default::default()
            },
        ),
        SourceSchema::Native => (
            None,
            sql_defined_pk_names,
            StreamSourceInfo {
                row_format: RowFormatType::Native as i32,
                ..Default::default()
            },
        ),
        SourceSchema::Bytes => {
            if !sql_defined_schema || sql_defined_columns.len() != 1 {
                return Err(RwError::from(ProtocolError(
                    "BYTES format only accepts one column".to_string(),
                )));
            }

            match sql_defined_columns[0].data_type {
                Some(ast::DataType::Bytea) => {}
                _ => {
                    return Err(RwError::from(ProtocolError(
                        "BYTES format only accepts BYTEA type".to_string(),
                    )))
                }
            }

            (
                None,
                sql_defined_pk_names,
                StreamSourceInfo {
                    row_format: RowFormatType::Bytes as i32,
                    ..Default::default()
                },
            )
        }
        SourceSchema::DebeziumMongoJson => {
            let mut columns = vec![
                ColumnCatalog {
                    column_desc: ColumnDesc {
                        data_type: DataType::Varchar,
                        column_id: 0.into(),
                        name: "_id".to_string(),
                        field_descs: vec![],
                        type_name: "".to_string(),
                        generated_or_default_column: None,
                    },
                    is_hidden: false,
                },
                ColumnCatalog {
                    column_desc: ColumnDesc {
                        data_type: DataType::Jsonb,
                        column_id: 0.into(),
                        name: "payload".to_string(),
                        field_descs: vec![],
                        type_name: "".to_string(),
                        generated_or_default_column: None,
                    },
                    is_hidden: false,
                },
            ];
            if sql_defined_schema {
                let non_generated_sql_defined_columns =
                    non_generated_sql_columns(sql_defined_columns);
                if non_generated_sql_defined_columns.len() != 2
                    && non_generated_sql_defined_columns[0].name.real_value() != columns[0].name()
                    && non_generated_sql_defined_columns[1].name.real_value() != columns[1].name()
                {
                    return Err(RwError::from(ProtocolError(
                        "the not generated columns of the source with row format DebeziumMongoJson
                         must be (_id [Jsonb | Varchar | Int32 | Int64], payload jsonb)."
                            .to_string(),
                    )));
                }
                if let Some(key_data_type) = &non_generated_sql_defined_columns[0].data_type {
                    let key_data_type = bind_data_type(key_data_type)?;
                    match key_data_type {
                        DataType::Jsonb | DataType::Varchar | DataType::Int32 | DataType::Int64 => {
                            columns[0].column_desc.data_type = key_data_type;
                        }
                        _ => {
                            return Err(RwError::from(ProtocolError(
                                "the `_id` column of the source with row format DebeziumMongoJson
                             must be [Jsonb | Varchar | Int32 | Int64]"
                                    .to_string(),
                            )));
                        }
                    }
                }
                if let Some(value_data_type) = &non_generated_sql_defined_columns[1].data_type {
                    if !matches!(bind_data_type(value_data_type)?, DataType::Jsonb) {
                        return Err(RwError::from(ProtocolError(
                            "the `payload` column of the source with row format DebeziumMongoJson
                             must be Jsonb datatype"
                                .to_string(),
                        )));
                    }
                }
            }
            let pk_names = if sql_defined_pk {
                sql_defined_pk_names
            } else {
                vec!["_id".to_string()]
            };

            (
                Some(columns),
                pk_names,
                StreamSourceInfo {
                    row_format: RowFormatType::DebeziumMongoJson as i32,
                    ..Default::default()
                },
            )
        }
    })
}

// Add a hidden column `_rw_kafka_timestamp` to each message from Kafka source.
fn check_and_add_timestamp_column(
    with_properties: &HashMap<String, String>,
    columns: &mut Vec<ColumnCatalog>,
) {
    if is_kafka_connector(with_properties) {
        let kafka_timestamp_column = ColumnCatalog {
            column_desc: ColumnDesc {
                data_type: DataType::Timestamptz,
                column_id: ColumnId::placeholder(),
                name: KAFKA_TIMESTAMP_COLUMN_NAME.to_string(),
                field_descs: vec![],
                type_name: "".to_string(),
                generated_or_default_column: None,
            },

            is_hidden: true,
        };
        columns.push(kafka_timestamp_column);
    }
}

pub(super) fn bind_source_watermark(
    session: &SessionImpl,
    name: String,
    source_watermarks: Vec<SourceWatermark>,
    column_catalogs: &[ColumnCatalog],
) -> Result<Vec<WatermarkDesc>> {
    let mut binder = Binder::new_for_ddl(session);
    binder.bind_columns_to_context(name.clone(), column_catalogs.to_vec())?;

    let watermark_descs = source_watermarks
        .into_iter()
        .map(|source_watermark| {
            let col_name = source_watermark.column.real_value();
            let watermark_idx = binder.get_column_binding_index(name.clone(), &col_name)?;

            let expr = binder.bind_expr(source_watermark.expr)?.to_expr_proto();

            Ok::<_, RwError>(WatermarkDesc {
                watermark_idx: watermark_idx as u32,
                expr: Some(expr),
            })
        })
        .try_collect()?;
    Ok(watermark_descs)
}

static CONNECTORS_COMPATIBLE_FORMATS: LazyLock<HashMap<String, Vec<RowFormatType>>> = LazyLock::new(
    || {
        convert_args!(hashmap!(
                KAFKA_CONNECTOR => vec![RowFormatType::Csv, RowFormatType::Json, RowFormatType::Protobuf, RowFormatType::DebeziumJson, RowFormatType::Avro, RowFormatType::Maxwell, RowFormatType::CanalJson, RowFormatType::DebeziumAvro,RowFormatType::DebeziumMongoJson, RowFormatType::UpsertJson, RowFormatType::UpsertAvro, RowFormatType::Bytes],
                PULSAR_CONNECTOR => vec![RowFormatType::Json, RowFormatType::Protobuf, RowFormatType::DebeziumJson, RowFormatType::Avro, RowFormatType::Maxwell, RowFormatType::CanalJson, RowFormatType::Bytes, RowFormatType::UpsertJson, RowFormatType::UpsertAvro],
                KINESIS_CONNECTOR => vec![RowFormatType::Json, RowFormatType::Protobuf, RowFormatType::DebeziumJson, RowFormatType::Avro, RowFormatType::Maxwell, RowFormatType::CanalJson, RowFormatType::Bytes, RowFormatType::UpsertJson, RowFormatType::UpsertAvro],
                GOOGLE_PUBSUB_CONNECTOR => vec![RowFormatType::Json, RowFormatType::Protobuf, RowFormatType::DebeziumJson, RowFormatType::Avro, RowFormatType::Maxwell, RowFormatType::CanalJson, RowFormatType::Bytes],
                NEXMARK_CONNECTOR => vec![RowFormatType::Native, RowFormatType::Bytes],
                DATAGEN_CONNECTOR => vec![RowFormatType::Native, RowFormatType::Json, RowFormatType::Bytes],
                S3_CONNECTOR => vec![RowFormatType::Csv, RowFormatType::Json],
                MYSQL_CDC_CONNECTOR => vec![RowFormatType::DebeziumJson, RowFormatType::Bytes],
                POSTGRES_CDC_CONNECTOR => vec![RowFormatType::DebeziumJson, RowFormatType::Bytes],
                CITUS_CDC_CONNECTOR => vec![RowFormatType::DebeziumJson, RowFormatType::Bytes],
        ))
    },
);

fn source_shema_to_row_format(source_schema: &SourceSchema) -> RowFormatType {
    match source_schema {
        SourceSchema::Avro(_) => RowFormatType::Avro,
        SourceSchema::Protobuf(_) => RowFormatType::Protobuf,
        SourceSchema::Json => RowFormatType::Json,
        SourceSchema::DebeziumJson => RowFormatType::DebeziumJson,
        SourceSchema::DebeziumMongoJson => RowFormatType::DebeziumMongoJson,
        SourceSchema::DebeziumAvro(_) => RowFormatType::DebeziumAvro,
        SourceSchema::UpsertJson => RowFormatType::UpsertJson,
        SourceSchema::UpsertAvro(_) => RowFormatType::UpsertAvro,
        SourceSchema::Maxwell => RowFormatType::Maxwell,
        SourceSchema::CanalJson => RowFormatType::CanalJson,
        SourceSchema::Csv(_) => RowFormatType::Csv,
        SourceSchema::Native => RowFormatType::Native,
        SourceSchema::Bytes => RowFormatType::Bytes,
    }
}

pub fn validate_compatibility(
    source_schema: &SourceSchema,
    props: &mut HashMap<String, String>,
) -> Result<()> {
    let connector = get_connector(props)
        .ok_or_else(|| RwError::from(ProtocolError("missing field 'connector'".to_string())))?;
    let row_format = source_shema_to_row_format(source_schema);

    let compatible_formats = CONNECTORS_COMPATIBLE_FORMATS
        .get(&connector)
        .ok_or_else(|| {
            RwError::from(ProtocolError(format!(
                "connector {} is not supported",
                connector
            )))
        })?;
    if connector != KAFKA_CONNECTOR
        && matches!(
            &source_schema,
            SourceSchema::Protobuf(ProtobufSchema {
                use_schema_registry: true,
                ..
            }) | SourceSchema::Avro(AvroSchema {
                use_schema_registry: true,
                ..
            }) | SourceSchema::DebeziumAvro(_)
        )
    {
        return Err(RwError::from(ProtocolError(format!(
            "The {} must be kafka when schema registry is used",
            UPSTREAM_SOURCE_KEY
        ))));
    }
    if !compatible_formats.contains(&row_format) {
        return Err(RwError::from(ProtocolError(format!(
            "connector {} does not support row format {:?}",
            connector, row_format
        ))));
    }

    if connector == POSTGRES_CDC_CONNECTOR {
        if !props.contains_key("slot.name") {
            // Build a random slot name with UUID
            // e.g. "rw_cdc_f9a3567e6dd54bf5900444c8b1c03815"
            let uuid = uuid::Uuid::new_v4().to_string().replace('-', "");
            props.insert("slot.name".into(), format!("rw_cdc_{}", uuid));
        }
        if !props.contains_key("schema.name") {
            // Default schema name is "public"
            props.insert("schema.name".into(), "public".into());
        }
        if !props.contains_key("publication.name") {
            // Default publication name is "rw_publication"
            props.insert("publication.name".into(), "rw_publication".into());
        }
        if !props.contains_key("publication.create.enable") {
            // Default auto create publication if doesn't exist
            props.insert("publication.create.enable".into(), "true".into());
        }
    }
    Ok(())
}

/// Performs early stage checking in frontend to see if the schema of the given `columns` is
/// compatible with the connector extracted from the properties. Currently this only works for
/// `nexmark` connector since it's in chunk format.
///
/// One should only call this function after all properties of all columns are resolved, like
/// generated column descriptors.
pub(super) fn check_source_schema(
    props: &HashMap<String, String>,
    row_id_index: Option<usize>,
    columns: &[ColumnCatalog],
) -> Result<()> {
    let Some(connector) = get_connector(props) else {
        return Ok(());
    };

    if connector != NEXMARK_CONNECTOR {
        return Ok(());
    }

    let table_type = props
        .get("nexmark.table.type")
        .map(|t| t.to_ascii_lowercase());

    let event_type = match table_type.as_deref() {
        None => None,
        Some("bid") => Some(EventType::Bid),
        Some("auction") => Some(EventType::Auction),
        Some("person") => Some(EventType::Person),
        Some(t) => {
            return Err(RwError::from(ProtocolError(format!(
                "unsupported table type for nexmark source: {}",
                t
            ))))
        }
    };

    // Ignore the generated columns and map the index of row_id column.
    let user_defined_columns = columns.iter().filter(|c| !c.is_generated());
    let row_id_index = if let Some(index) = row_id_index {
        let col_id = columns[index].column_id();
        user_defined_columns
            .clone()
            .position(|c| c.column_id() == col_id)
            .unwrap()
            .into()
    } else {
        None
    };

    let expected = get_event_data_types_with_names(event_type, row_id_index);
    let user_defined = user_defined_columns
        .map(|c| {
            (
                c.column_desc.name.to_ascii_lowercase(),
                c.column_desc.data_type.to_owned(),
            )
        })
        .collect_vec();

    if expected != user_defined {
        let cmp = pretty_assertions::Comparison::new(&expected, &user_defined);
        return Err(RwError::from(ProtocolError(format!(
            "The schema of the nexmark source must specify all columns in order:\n{cmp}",
        ))));
    }
    Ok(())
}

pub async fn handle_create_source(
    handler_args: HandlerArgs,
    stmt: CreateSourceStatement,
) -> Result<RwPgResponse> {
    let session = handler_args.session.clone();

    session.check_relation_name_duplicated(stmt.source_name.clone())?;

    let db_name = session.database();
    let (schema_name, name) = Binder::resolve_schema_qualified_name(db_name, stmt.source_name)?;
    let (database_id, schema_id) =
        session.get_database_and_schema_id_for_create(schema_name.clone())?;

    if handler_args.with_options.is_empty() {
        return Err(RwError::from(InvalidInputSyntax(
            "missing WITH clause".to_string(),
        )));
    }

    if let CompatibleSourceSchema::V2(s) = &stmt.source_schema {
        if s.row_encode == Encode::Json && stmt.columns.is_empty() {
            return Err(RwError::from(InvalidInputSyntax(
                "schema definition is required for ENCODE JSON".to_owned(),
            )));
        }
    }

    let (source_schema, _, notice) = stmt
        .source_schema
        .into_source_schema()
        .map_err(|e| ErrorCode::InvalidInputSyntax(e.inner_msg()))?;
    if let Some(notice) = notice {
        session.notice_to_user(notice)
    }
    let mut with_properties = handler_args.with_options.into_inner().into_iter().collect();
    validate_compatibility(&source_schema, &mut with_properties)?;

    ensure_table_constraints_supported(&stmt.constraints)?;
    let pk_names = bind_pk_names(&stmt.columns, &stmt.constraints)?;

    let (columns_from_resolve_source, pk_names, source_info) =
        try_bind_columns_from_source(&source_schema, pk_names, &stmt.columns, &with_properties)
            .await?;
    let columns_from_sql = bind_sql_columns(&stmt.columns)?;

    let mut columns = columns_from_resolve_source.unwrap_or(columns_from_sql);

    check_and_add_timestamp_column(&with_properties, &mut columns);

    let mut col_id_gen = ColumnIdGenerator::new_initial();
    for c in &mut columns {
        c.column_desc.column_id = col_id_gen.generate(c.name())
    }

    if !pk_names.is_empty() {
        return Err(ErrorCode::InvalidInputSyntax(
            "Source does not support PRIMARY KEY constraint, please use \"CREATE TABLE\" instead"
                .to_owned(),
        )
        .into());
    }

    let (mut columns, pk_column_ids, row_id_index) = bind_pk_on_relation(columns, pk_names)?;

    debug_assert!(is_column_ids_dedup(&columns));

    let watermark_descs =
        bind_source_watermark(&session, name.clone(), stmt.source_watermarks, &columns)?;
    // TODO(yuhao): allow multiple watermark on source.
    assert!(watermark_descs.len() <= 1);

    bind_sql_column_constraints(&session, name.clone(), &mut columns, stmt.columns)?;

    check_source_schema(&with_properties, row_id_index, &columns)?;

    let row_id_index = row_id_index.map(|index| index as _);
    let pk_column_ids = pk_column_ids.into_iter().map(Into::into).collect();

    let columns = columns.into_iter().map(|c| c.to_protobuf()).collect_vec();

    // resolve privatelink connection for Kafka source
    let mut with_options = WithOptions::new(with_properties);
    let connection_id =
        resolve_connection_in_with_option(&mut with_options, &schema_name, &session)?;
    let definition = handler_args.normalized_sql;

    let source = PbSource {
        id: TableId::placeholder().table_id,
        schema_id,
        database_id,
        name,
        row_id_index,
        columns,
        pk_column_ids,
        properties: with_options.into_inner().into_iter().collect(),
        info: Some(source_info),
        owner: session.user_id(),
        watermark_descs,
        definition,
        connection_id,
        optional_associated_table_id: None,
    };

    let catalog_writer = session.catalog_writer()?;
    catalog_writer.create_source(source).await?;

    Ok(PgResponse::empty_result(StatementType::CREATE_SOURCE))
}

#[cfg(test)]
pub mod tests {
    use std::collections::HashMap;

    use risingwave_common::catalog::{
        row_id_column_name, DEFAULT_DATABASE_NAME, DEFAULT_SCHEMA_NAME,
    };
    use risingwave_common::types::DataType;

    use crate::catalog::root_catalog::SchemaPath;
    use crate::test_utils::{create_proto_file, LocalFrontend, PROTO_FILE_DATA};

    #[tokio::test]
    async fn test_create_source_handler() {
        let proto_file = create_proto_file(PROTO_FILE_DATA);
        let sql = format!(
            r#"CREATE SOURCE t
    WITH (connector = 'kinesis')
    FORMAT PLAIN ENCODE PROTOBUF (message = '.test.TestRecord', schema.location = 'file://{}')"#,
            proto_file.path().to_str().unwrap()
        );
        let frontend = LocalFrontend::new(Default::default()).await;
        frontend.run_sql(sql).await.unwrap();

        let session = frontend.session_ref();
        let catalog_reader = session.env().catalog_reader().read_guard();
        let schema_path = SchemaPath::Name(DEFAULT_SCHEMA_NAME);

        // Check source exists.
        let (source, _) = catalog_reader
            .get_source_by_name(DEFAULT_DATABASE_NAME, schema_path, "t")
            .unwrap();
        assert_eq!(source.name, "t");

        let columns = source
            .columns
            .iter()
            .map(|col| (col.name(), col.data_type().clone()))
            .collect::<HashMap<&str, DataType>>();

        let city_type = DataType::new_struct(
            vec![DataType::Varchar, DataType::Varchar],
            vec!["address".to_string(), "zipcode".to_string()],
        );
        let row_id_col_name = row_id_column_name();
        let expected_columns = maplit::hashmap! {
            row_id_col_name.as_str() => DataType::Serial,
            "id" => DataType::Int32,
            "zipcode" => DataType::Int64,
            "rate" => DataType::Float32,
            "country" => DataType::new_struct(
                vec![DataType::Varchar,city_type,DataType::Varchar],
                vec!["address".to_string(), "city".to_string(), "zipcode".to_string()],
            ),
        };
        assert_eq!(columns, expected_columns);
    }
}
