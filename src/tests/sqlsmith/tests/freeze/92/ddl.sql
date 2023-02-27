CREATE TABLE supplier (s_suppkey INT, s_name CHARACTER VARYING, s_address CHARACTER VARYING, s_nationkey INT, s_phone CHARACTER VARYING, s_acctbal NUMERIC, s_comment CHARACTER VARYING, PRIMARY KEY (s_suppkey));
CREATE TABLE part (p_partkey INT, p_name CHARACTER VARYING, p_mfgr CHARACTER VARYING, p_brand CHARACTER VARYING, p_type CHARACTER VARYING, p_size INT, p_container CHARACTER VARYING, p_retailprice NUMERIC, p_comment CHARACTER VARYING, PRIMARY KEY (p_partkey));
CREATE TABLE partsupp (ps_partkey INT, ps_suppkey INT, ps_availqty INT, ps_supplycost NUMERIC, ps_comment CHARACTER VARYING, PRIMARY KEY (ps_partkey, ps_suppkey));
CREATE TABLE customer (c_custkey INT, c_name CHARACTER VARYING, c_address CHARACTER VARYING, c_nationkey INT, c_phone CHARACTER VARYING, c_acctbal NUMERIC, c_mktsegment CHARACTER VARYING, c_comment CHARACTER VARYING, PRIMARY KEY (c_custkey));
CREATE TABLE orders (o_orderkey BIGINT, o_custkey INT, o_orderstatus CHARACTER VARYING, o_totalprice NUMERIC, o_orderdate DATE, o_orderpriority CHARACTER VARYING, o_clerk CHARACTER VARYING, o_shippriority INT, o_comment CHARACTER VARYING, PRIMARY KEY (o_orderkey));
CREATE TABLE lineitem (l_orderkey BIGINT, l_partkey INT, l_suppkey INT, l_linenumber INT, l_quantity NUMERIC, l_extendedprice NUMERIC, l_discount NUMERIC, l_tax NUMERIC, l_returnflag CHARACTER VARYING, l_linestatus CHARACTER VARYING, l_shipdate DATE, l_commitdate DATE, l_receiptdate DATE, l_shipinstruct CHARACTER VARYING, l_shipmode CHARACTER VARYING, l_comment CHARACTER VARYING, PRIMARY KEY (l_orderkey, l_linenumber));
CREATE TABLE nation (n_nationkey INT, n_name CHARACTER VARYING, n_regionkey INT, n_comment CHARACTER VARYING, PRIMARY KEY (n_nationkey));
CREATE TABLE region (r_regionkey INT, r_name CHARACTER VARYING, r_comment CHARACTER VARYING, PRIMARY KEY (r_regionkey));
CREATE TABLE person (id BIGINT, name CHARACTER VARYING, email_address CHARACTER VARYING, credit_card CHARACTER VARYING, city CHARACTER VARYING, state CHARACTER VARYING, date_time TIMESTAMP, extra CHARACTER VARYING, PRIMARY KEY (id));
CREATE TABLE auction (id BIGINT, item_name CHARACTER VARYING, description CHARACTER VARYING, initial_bid BIGINT, reserve BIGINT, date_time TIMESTAMP, expires TIMESTAMP, seller BIGINT, category BIGINT, extra CHARACTER VARYING, PRIMARY KEY (id));
CREATE TABLE bid (auction BIGINT, bidder BIGINT, price BIGINT, channel CHARACTER VARYING, url CHARACTER VARYING, date_time TIMESTAMP, extra CHARACTER VARYING);
CREATE TABLE alltypes1 (c1 BOOLEAN, c2 SMALLINT, c3 INT, c4 BIGINT, c5 REAL, c6 DOUBLE, c7 NUMERIC, c8 DATE, c9 CHARACTER VARYING, c10 TIME, c11 TIMESTAMP, c13 INTERVAL, c14 STRUCT<a INT>, c15 INT[], c16 CHARACTER VARYING[]);
CREATE TABLE alltypes2 (c1 BOOLEAN, c2 SMALLINT, c3 INT, c4 BIGINT, c5 REAL, c6 DOUBLE, c7 NUMERIC, c8 DATE, c9 CHARACTER VARYING, c10 TIME, c11 TIMESTAMP, c13 INTERVAL, c14 STRUCT<a INT>, c15 INT[], c16 CHARACTER VARYING[]);
CREATE MATERIALIZED VIEW m0 AS SELECT t_1.s_phone AS col_0 FROM bid AS t_0 RIGHT JOIN supplier AS t_1 ON t_0.url = t_1.s_name AND true GROUP BY t_1.s_phone, t_1.s_comment, t_1.s_address, t_1.s_nationkey, t_1.s_suppkey, t_1.s_name HAVING false;
CREATE MATERIALIZED VIEW m1 AS SELECT TIMESTAMP '2022-12-08 18:34:29' AS col_0, TIMESTAMP '2022-12-08 18:35:29' AS col_1, (CASE WHEN true THEN sq_1.col_2 ELSE sq_1.col_2 END) AS col_2 FROM (SELECT (tumble_0.bidder >> (INT '7')) AS col_0, ((BIGINT '121') # (SMALLINT '32767')) AS col_1, (TIMESTAMP '2022-12-07 18:35:29') AS col_2 FROM tumble(bid, bid.date_time, INTERVAL '59') AS tumble_0 WHERE (((REAL '-2147483648') + (REAL '657')) <= (REAL '306')) GROUP BY tumble_0.bidder, tumble_0.channel, tumble_0.date_time) AS sq_1 WHERE false GROUP BY sq_1.col_2, sq_1.col_0;
CREATE MATERIALIZED VIEW m2 AS SELECT (FLOAT '533') AS col_0, t_2.c_comment AS col_1, (418) AS col_2, (INTERVAL '86400') AS col_3 FROM customer AS t_2 GROUP BY t_2.c_comment;
CREATE MATERIALIZED VIEW m4 AS SELECT (BIGINT '883') AS col_0, t_1.c15 AS col_1, t_1.c15 AS col_2, (916) AS col_3 FROM m0 AS t_0 LEFT JOIN alltypes2 AS t_1 ON t_0.col_0 = t_1.c9 AND (t_1.c3 >= (SMALLINT '4473')) GROUP BY t_1.c15 HAVING false;
CREATE MATERIALIZED VIEW m5 AS SELECT hop_0.reserve AS col_0 FROM hop(auction, auction.expires, INTERVAL '60', INTERVAL '2520') AS hop_0 WHERE true GROUP BY hop_0.category, hop_0.reserve, hop_0.item_name HAVING false;
CREATE MATERIALIZED VIEW m6 AS SELECT (OVERLAY(t_0.o_orderstatus PLACING 'AkDJGChn7q' FROM ((INT '0') % (INT '1')) FOR (INT '245'))) AS col_0, ('wmqMaPOjpD') AS col_1 FROM orders AS t_0 GROUP BY t_0.o_totalprice, t_0.o_orderstatus, t_0.o_clerk HAVING true;
CREATE MATERIALIZED VIEW m7 AS SELECT (1779396710) AS col_0, t_2.l_shipinstruct AS col_1, t_2.l_comment AS col_2 FROM lineitem AS t_2 GROUP BY t_2.l_orderkey, t_2.l_partkey, t_2.l_comment, t_2.l_tax, t_2.l_shipinstruct, t_2.l_discount;
CREATE MATERIALIZED VIEW m8 AS WITH with_0 AS (SELECT (TRIM(LEADING sq_4.col_1 FROM sq_4.col_1)) AS col_0, sq_4.col_1 AS col_1 FROM (SELECT t_3.n_comment AS col_0, (TRIM(t_3.n_comment)) AS col_1, 'HuD6n3xv2Z' AS col_2 FROM nation AS t_3 GROUP BY t_3.n_comment, t_3.n_name HAVING true) AS sq_4 GROUP BY sq_4.col_1 HAVING false) SELECT (INT '2147483647') AS col_0 FROM with_0 WHERE ((SMALLINT '430') <= ((FLOAT '-2147483648') * (FLOAT '971')));
CREATE MATERIALIZED VIEW m9 AS SELECT sq_2.col_0 AS col_0, sq_2.col_0 AS col_1, sq_2.col_0 AS col_2, DATE '2022-12-01' AS col_3 FROM (WITH with_0 AS (SELECT t_1.c11 AS col_0 FROM alltypes1 AS t_1 WHERE t_1.c1 GROUP BY t_1.c9, t_1.c16, t_1.c11) SELECT DATE '2022-12-08' AS col_0 FROM with_0) AS sq_2 WHERE ((FLOAT '585') >= (SMALLINT '255')) GROUP BY sq_2.col_0 HAVING false;
