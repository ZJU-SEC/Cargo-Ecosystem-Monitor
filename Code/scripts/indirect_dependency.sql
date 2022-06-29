
-- Indirect Dependency

---- Basic
-- crate <-> crate
SELECT COUNT(DISTINCT(crate_from)) FROM dep_crate;
SELECT COUNT(DISTINCT(crate_to)) FROM dep_crate;
-- version <-> version
SELECT COUNT(DISTINCT(version_from)) FROM dep_version;
SELECT COUNT(DISTINCT(version_to)) FROM dep_version;

---- Advanced
-- Version with most indirect dependency
SELECT version_from, COUNT(DISTINCT version_to) as indirect_dep FROM dep_version GROUP BY version_from ORDER BY indirect_dep desc LIMIT 100;

-- Version with most indirect dependents
SELECT version_to, COUNT(DISTINCT version_from) as indirect_dep FROM dep_version GROUP BY version_to ORDER BY indirect_dep desc LIMIT 100;

-- Version with most indirect dependents, full info
with indirect_deps AS(
SELECT version_to, COUNT(DISTINCT version_from) as indirect_dep 
FROM dep_version GROUP BY version_to ORDER BY indirect_dep desc LIMIT 100
),version_name AS (
SELECT name,versions.id as version_id, num as version_num  FROM versions INNER JOIN crates ON crates.id = versions.crate_id )
SELECT name, version_num , indirect_dep
FROM version_name INNER JOIN indirect_deps ON indirect_deps.version_to = version_name.version_id ;

-- Crate with accumulative crates with most indirect dependents
DROP TABLE IF EXISTS tmp_hot_dep_crates,accumulate_hot_crates, accumulate_hot_crates_near10;
CREATE TEMP TABLE tmp_hot_dep_crates AS
(SELECT crate_to, COUNT(DISTINCT crate_from) AS total_dependents FROM dep_crate
GROUP BY crate_to ORDER BY total_dependents desc LIMIT 100);
CREATE TABLE accumulate_hot_crates(
    accumulative_num integer PRIMARY KEY,
    crates_count integer
);
do 
$$
declare
	N integer;
begin
	IF EXISTS (
    SELECT FROM 
        information_schema.tables 
    WHERE
        table_name = 'accumulate_hot_crates'
    )THEN
        for N in 1..100 loop
            INSERT INTO accumulate_hot_crates 
            SELECT N, COUNT(DISTINCT crate_from) AS total_dependents FROM dep_crate  WHERE crate_to 
            IN (SELECT crate_to FROM tmp_hot_dep_crates LIMIT N);
        end loop;
    END IF;
end; 
$$;
CREATE TABLE accumulate_hot_crates_near10(
    near_accumulative_num integer PRIMARY KEY,
    crates_count integer
);
-- 10 Near Accumulative Hot Crates by Indir Dependents of TOP `N=50`
do 
$$
declare
	N integer;
begin
	IF EXISTS (
    SELECT FROM 
        information_schema.tables 
    WHERE
        table_name = 'accumulate_hot_crates_near10'
    )THEN
        for N in 0..50 loop
            INSERT INTO accumulate_hot_crates_near10 
            SELECT N, COUNT(DISTINCT crate_from) AS total_dependents FROM dep_crate  WHERE crate_to 
            IN (SELECT crate_to FROM tmp_hot_dep_crates OFFSET N LIMIT 10);
        end loop;
    END IF;
end; 
$$;




-- Advisory Propagation
SELECT COUNT(DISTINCT version_from) FROM dep_version WHERE version_to IN (SELECT * FROM advisory);



-- "=version" Propagation (rough)
SELECT COUNT(DISTINCT version_from) FROM dep_version WHERE version_to IN
(SELECT id  FROM dependencies WHERE req LIKE '=%' AND optional = false);
