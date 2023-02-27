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
CREATE MATERIALIZED VIEW m0 AS SELECT ((INT '251')) AS col_0 FROM person AS t_0 JOIN supplier AS t_1 ON t_0.state = t_1.s_name GROUP BY t_0.date_time HAVING false;
CREATE MATERIALIZED VIEW m1 AS SELECT (CASE WHEN false THEN t_0.o_totalprice ELSE t_0.o_totalprice END) AS col_0, ((839)) AS col_1 FROM orders AS t_0 WHERE false GROUP BY t_0.o_totalprice HAVING ((FLOAT '833') = (CASE WHEN false THEN (CASE WHEN max(false) THEN (REAL '167') ELSE (REAL '926') END) ELSE (REAL '1') END));
CREATE MATERIALIZED VIEW m2 AS SELECT hop_0.c2 AS col_0, TIMESTAMP '2022-10-09 00:01:15' AS col_1, (INTERVAL '1') AS col_2 FROM hop(alltypes2, alltypes2.c11, INTERVAL '60', INTERVAL '3540') AS hop_0 GROUP BY hop_0.c8, hop_0.c6, hop_0.c2, hop_0.c10, hop_0.c11;
CREATE MATERIALIZED VIEW m3 AS SELECT TIMESTAMP '2022-10-09 00:01:15' AS col_0 FROM bid AS t_0 RIGHT JOIN bid AS t_1 ON t_0.auction = t_1.bidder GROUP BY t_1.date_time, t_0.extra, t_1.channel, t_0.date_time, t_1.price, t_1.url;
CREATE MATERIALIZED VIEW m4 AS SELECT (-2147483648) AS col_0 FROM part AS t_0 GROUP BY t_0.p_name, t_0.p_partkey, t_0.p_mfgr, t_0.p_comment;
CREATE MATERIALIZED VIEW m5 AS SELECT tumble_0.url AS col_0 FROM tumble(bid, bid.date_time, INTERVAL '87') AS tumble_0 WHERE (true) GROUP BY tumble_0.url;
CREATE MATERIALIZED VIEW m6 AS SELECT hop_0.col_0 AS col_0 FROM hop(m3, m3.col_0, INTERVAL '60', INTERVAL '3660') AS hop_0 GROUP BY hop_0.col_0;
CREATE MATERIALIZED VIEW m7 AS SELECT (BIGINT '9223372036854775807') AS col_0 FROM hop(bid, bid.date_time, INTERVAL '353132', INTERVAL '33547540') AS hop_0 WHERE true GROUP BY hop_0.bidder HAVING true;
