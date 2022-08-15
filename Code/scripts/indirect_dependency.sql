
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

-- Crates with most indirect dependents, full info
with indirect_deps AS(
SELECT crate_to, COUNT(DISTINCT crate_from) as indirect_dep 
FROM dep_crate GROUP BY crate_to ORDER BY indirect_dep desc LIMIT 100
)
SELECT name, indirect_dep
FROM crates INNER JOIN indirect_deps ON indirect_deps.crate_to = crates.id ;


-- Crate "Libc" different level indirect "version" dependents
SELECT * FROM dep_version WHERE version_to IN 
(SELECT id as version_id FROM versions WHERE crate_id = 795) 

-- Crate "Libc" different level indirect "crate" dependents
WITH libc_version_indir_dep AS 
(SELECT * FROM dep_version WHERE version_to IN 
(SELECT id as version_id FROM versions WHERE crate_id = 795))
SELECT DISTINCT crate_id, dep_level FROM versions INNER JOIN libc_version_indir_dep ON version_from = id;

-- Crate "Libc" different level indirect "crate" dependents with their names
WITH libc_version_indir_dep_crate AS
(WITH libc_version_indir_dep AS 
(SELECT * FROM dep_version WHERE version_to IN 
(SELECT id as version_id FROM versions WHERE crate_id = 795))
SELECT DISTINCT crate_id, dep_level FROM versions INNER JOIN libc_version_indir_dep ON version_from = id)
SELECT  crate_id, name,dep_level FROM crates INNER JOIN libc_version_indir_dep_crate ON crates.id = crate_id
-- Crate "Libc" different level indirect "crate" dependents with their names and min dep_level
WITH libc_version_indir_dep_crate AS
(WITH libc_version_indir_dep AS 
(SELECT * FROM dep_version WHERE version_to IN 
(SELECT id as version_id FROM versions WHERE crate_id = 795))
SELECT DISTINCT crate_id, dep_level FROM versions INNER JOIN libc_version_indir_dep ON version_from = id)
SELECT  crate_id, name, MIN(dep_level) FROM crates INNER JOIN libc_version_indir_dep_crate ON crates.id = crate_id
GROUP BY crate_id, name
-- Top 5 crates(dep) different level indirect "crate" dependents with their names and min dep_level
DROP TABLE IF EXISTS tmp_hot_dep_crates;
CREATE TABLE tmp_hot_dep_crates AS
(SELECT crate_to, COUNT(DISTINCT crate_from) AS total_dependents FROM dep_crate
GROUP BY crate_to ORDER BY total_dependents desc LIMIT 5);
WITH libc_version_indir_dep_crate AS
(WITH libc_version_indir_dep AS 
(SELECT * FROM dep_version WHERE version_to IN (
    SELECT versions.id FROM tmp_hot_dep_crates INNER JOIN versions ON crate_to=crate_id))
SELECT DISTINCT crate_id, dep_level FROM versions INNER JOIN libc_version_indir_dep ON version_from = id)
SELECT  crate_id, name, MIN(dep_level) FROM crates INNER JOIN libc_version_indir_dep_crate ON crates.id = crate_id
GROUP BY crate_id, name;
-- TOP Nth crates(dep) different level 
-- indirect "crate" dependents with their names and min dep_level
DROP TABLE IF EXISTS tmp_hot_dep_crates;
CREATE TABLE tmp_hot_dep_crates AS
(SELECT crate_to, COUNT(DISTINCT crate_from) AS total_dependents FROM dep_crate
GROUP BY crate_to ORDER BY total_dependents desc LIMIT 1 OFFSET (N-1));
WITH libc_version_indir_dep_crate AS
(WITH libc_version_indir_dep AS 
(SELECT * FROM dep_version WHERE version_to IN (
    SELECT versions.id FROM tmp_hot_dep_crates INNER JOIN versions ON crate_to=crate_id))
SELECT DISTINCT crate_id, dep_level FROM versions INNER JOIN libc_version_indir_dep ON version_from = id)
SELECT  crate_id, name, MIN(dep_level) FROM crates INNER JOIN libc_version_indir_dep_crate ON crates.id = crate_id
GROUP BY crate_id, name;

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

-- Hot Owner by Indir Dependents (Need to build table `dep_crate` first)
WITH hot_owner AS 
(SELECT owner_id, COUNT(DISTINCT crate_from) AS total_dependents FROM crate_owners INNER JOIN dep_crate ON crate_id=crate_to GROUP BY owner_id)
SELECT name, total_dependents, gh_login as GithubAccount, gh_avatar as GithubAvatar, gh_id as GithubID 
FROM hot_owner INNER JOIN users ON owner_id =id ORDER BY total_dependents desc

-- Accumulative Hot Owner by Indir Dependents of TOP `N=50` (`N`<=50)
-- ATTENTION: It uses table `hot_owner`. 
DROP TABLE IF EXISTS tmp_owner_indir_crate,tmp_hot_owner_id,accumulate_hot_owners,accumulate_hot_owners_10near;
CREATE TEMP TABLE tmp_owner_indir_crate AS
(SELECT DISTINCT owner_id,  crate_from  FROM crate_owners INNER JOIN dep_crate ON crate_id=crate_to);
CREATE TEMP TABLE tmp_hot_owner_id AS
(SELECT owner_id, COUNT(DISTINCT crate_from) AS total_dependents FROM tmp_owner_indir_crate 
GROUP BY owner_id ORDER BY total_dependents desc LIMIT 100);
CREATE TABLE accumulate_hot_owners(
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
        table_name = 'accumulate_hot_owners'
    )THEN
        for N in 1..100 loop
            INSERT INTO accumulate_hot_owners 
            SELECT N, COUNT(DISTINCT crate_from) AS total_dependents FROM tmp_owner_indir_crate  WHERE owner_id 
            IN (SELECT owner_id FROM tmp_hot_owner_id LIMIT N);
        end loop;
    END IF;
end; 
$$;

CREATE TABLE accumulate_hot_owners_10near(
    near_accumulative_num integer PRIMARY KEY,
    crates_count integer
);
-- 10 Near Accumulative Hot Owner by Indir Dependents of TOP `N=50`
do 
$$
declare
	N integer;
begin
	IF EXISTS (
    SELECT FROM 
        information_schema.tables 
    WHERE
        table_name = 'accumulate_hot_owners_10near'
    )THEN
        for N in 0..50 loop
            INSERT INTO accumulate_hot_owners_10near 
            SELECT N, COUNT(DISTINCT crate_from) AS total_dependents FROM tmp_owner_indir_crate  WHERE owner_id 
            IN (SELECT owner_id FROM tmp_hot_owner_id OFFSET N LIMIT 10);
        end loop;
    END IF;
end; 
$$;







-- Advisory Propagation
SELECT COUNT(DISTINCT version_from) FROM dep_version WHERE version_to IN (SELECT * FROM advisory);
-- Advisory Propagation with Advisory Category
SELECT COUNT(DISTINCT version_from) FROM dep_version 
WHERE version_to IN (SELECT version_id FROM advisory WHERE categories like '%thread-safety%');
-- Advisory Version Count with Advisory Category
SELECT COUNT(DISTINCT version_id) FROM advisory WHERE categories like '%thread-safety%';

-- "=version" Propagation (rough)
SELECT COUNT(DISTINCT version_from) FROM dep_version WHERE version_to IN
(SELECT id  FROM dependencies WHERE req LIKE '=%' AND optional = false);


-- Max depth of indir dependencies of each version
SELECT version_to, MAX(dep_level) FROM dep_version GROUP BY version_to;
-- Max depth of indir dependencies of each version, ecosystem overview
WITH max_depth_version AS 
(SELECT version_to, MAX(dep_level) as max_dep FROM dep_version GROUP BY version_to)
SELECT max_dep, COUNT(max_dep) FROM max_depth_version GROUP BY max_dep ORDER BY max_dep asc


-- Evaluation
-- 1. Find crates with most indirect dependencies 
SELECT version_from, COUNT( DISTINCT version_to) as indir_dep FROM dep_version 
GROUP BY version_from ORDER BY indir_dep desc
-- 2. Get their indirect dependency
WITH target_dep AS(
WITH target_version AS 
(SELECT distinct version_to FROM dep_version
WHERE version_from = xxx)
SELECT crate_id, num FROM target_version INNER JOIN versions ON version_to = id)
SELECT name, num FROM target_dep INNER JOIN crates ON crate_id = id ORDER BY num asc


SELECT * FROM accuracy_evaluation_status WHERE status = `unevaluated`