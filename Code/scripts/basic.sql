-- Basic Query
-- Version <-> Crates
SELECT name, num as version_num  FROM versions INNER JOIN crates ON crates.id = versions.crate_id LIMIT 100


-- Overview

-- Crate Overview by year
--652
SELECT COUNT(id)  FROM crates WHERE created_at < '2015-01-01'   LIMIT 100
--3717
SELECT COUNT(id)  FROM crates WHERE created_at < '2016-01-01'   LIMIT 100
--7407
SELECT COUNT(id)  FROM crates WHERE created_at < '2017-01-01'   LIMIT 100
--13065
SELECT COUNT(id)  FROM crates WHERE created_at < '2018-01-01'   LIMIT 100
--21401
SELECT COUNT(id)  FROM crates WHERE created_at < '2019-01-01'   LIMIT 100
--33764
SELECT COUNT(id)  FROM crates WHERE created_at < '2020-01-01'   LIMIT 100
--51936
SELECT COUNT(id)  FROM crates WHERE created_at < '2021-01-01'   LIMIT 100
--73699
SELECT COUNT(id)  FROM crates WHERE created_at < '2022-01-01'   LIMIT 100
--77851
SELECT COUNT(id)  FROM crates WHERE created_at < '2022-03-01'   LIMIT 100

-- Version Overview by year
--1841
SELECT COUNT(id)  FROM versions  WHERE created_at < '2015-01-01'   LIMIT 100
--19928
SELECT COUNT(id)  FROM versions  WHERE created_at < '2016-01-01'   LIMIT 100
--40432
SELECT COUNT(id)  FROM versions  WHERE created_at < '2017-01-01'   LIMIT 100
--74503
SELECT COUNT(id)  FROM versions  WHERE created_at < '2018-01-01'   LIMIT 100
--122621
SELECT COUNT(id)  FROM versions  WHERE created_at < '2019-01-01'   LIMIT 100
--195765
SELECT COUNT(id)  FROM versions  WHERE created_at < '2020-01-01'   LIMIT 100
--315489
SELECT COUNT(id)  FROM versions  WHERE created_at < '2021-01-01'   LIMIT 100
--468715
SELECT COUNT(id)  FROM versions  WHERE created_at < '2022-01-01'   LIMIT 100
--501244
SELECT COUNT(id)  FROM versions  WHERE created_at < '2022-03-01'   LIMIT 100



