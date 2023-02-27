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
CREATE MATERIALIZED VIEW m0 AS SELECT (INTERVAL '86400') AS col_0 FROM lineitem AS t_0 RIGHT JOIN supplier AS t_1 ON t_0.l_discount = t_1.s_acctbal GROUP BY t_0.l_orderkey, t_0.l_shipdate HAVING false;
CREATE MATERIALIZED VIEW m1 AS SELECT sq_2.col_3 AS col_0, sq_2.col_0 AS col_1, sq_2.col_3 AS col_2, true AS col_3 FROM (SELECT 'YI5qnQrjvk' AS col_0, '0SA6dHqI3n' AS col_1, t_0.ps_comment AS col_2, t_0.ps_comment AS col_3 FROM partsupp AS t_0 RIGHT JOIN part AS t_1 ON t_0.ps_partkey = t_1.p_size WHERE false GROUP BY t_0.ps_comment) AS sq_2 GROUP BY sq_2.col_0, sq_2.col_3;
CREATE MATERIALIZED VIEW m2 AS SELECT tumble_0.category AS col_0, tumble_0.seller AS col_1, tumble_0.reserve AS col_2, tumble_0.category AS col_3 FROM tumble(auction, auction.date_time, INTERVAL '47') AS tumble_0 GROUP BY tumble_0.reserve, tumble_0.description, tumble_0.seller, tumble_0.category, tumble_0.date_time;
CREATE MATERIALIZED VIEW m3 AS WITH with_0 AS (SELECT (INT '524') AS col_0, (INT '181') AS col_1, (CASE WHEN false THEN t_2.c_custkey WHEN false THEN (INT '684') ELSE (INT '968') END) AS col_2 FROM alltypes2 AS t_1 JOIN customer AS t_2 ON t_1.c9 = t_2.c_phone WHERE ((t_1.c10 + t_1.c8) < t_1.c8) GROUP BY t_2.c_custkey HAVING false) SELECT TIMESTAMP '2022-07-27 13:54:49' AS col_0, (0) AS col_1 FROM with_0;
CREATE MATERIALIZED VIEW m4 AS SELECT (INTERVAL '561581') AS col_0, ((REAL '733') - (REAL '1197868537')) AS col_1 FROM hop(alltypes1, alltypes1.c11, INTERVAL '604800', INTERVAL '21772800') AS hop_0 WHERE hop_0.c1 GROUP BY hop_0.c13, hop_0.c4;
CREATE MATERIALIZED VIEW m5 AS SELECT t_1.s_name AS col_0 FROM m1 AS t_0 RIGHT JOIN supplier AS t_1 ON t_0.col_2 = t_1.s_phone GROUP BY t_1.s_name, t_1.s_address HAVING true;
CREATE MATERIALIZED VIEW m6 AS SELECT ((SMALLINT '0') - (hop_0.auction % (SMALLINT '-28762'))) AS col_0, (BIGINT '0') AS col_1, (md5(string_agg((concat('XavdxATsZ0')), hop_0.extra) FILTER(WHERE true))) AS col_2 FROM hop(bid, bid.date_time, INTERVAL '303019', INTERVAL '26362653') AS hop_0 GROUP BY hop_0.auction, hop_0.extra, hop_0.url HAVING min(true) FILTER(WHERE (false));
CREATE MATERIALIZED VIEW m7 AS SELECT (856) AS col_0 FROM customer AS t_0 FULL JOIN m5 AS t_1 ON t_0.c_comment = t_1.col_0 GROUP BY t_0.c_address, t_0.c_acctbal, t_0.c_nationkey, t_0.c_comment, t_0.c_custkey;
CREATE MATERIALIZED VIEW m8 AS SELECT sq_2.col_2 AS col_0, sq_2.col_2 AS col_1 FROM (SELECT t_1.col_1 AS col_0, 'dRAJO0GhTn' AS col_1, 'gOAtooif9F' AS col_2, 'MOesdLMUyx' AS col_3 FROM m6 AS t_0 JOIN m1 AS t_1 ON t_0.col_2 = t_1.col_2 AND t_1.col_3 GROUP BY t_1.col_1 HAVING min(true) FILTER(WHERE CAST((INT '2147483647') AS BOOLEAN))) AS sq_2 WHERE false GROUP BY sq_2.col_2 HAVING true;
CREATE MATERIALIZED VIEW m9 AS SELECT min(CAST(true AS INT)) AS col_0, t_1.p_container AS col_1 FROM bid AS t_0 JOIN part AS t_1 ON t_0.channel = t_1.p_brand GROUP BY t_0.extra, t_1.p_size, t_0.channel, t_1.p_name, t_0.bidder, t_1.p_container HAVING true;
