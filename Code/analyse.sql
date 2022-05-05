-- Sql scripts tools
-- Resolved max crate_id
SELECT MAX(crate_id) FROM dep_version INNER JOIN versions on dep_version.version_from=versions.id
-- Find Current Max resolved Offset
with max_crate as (SELECT MAX(crate_id) 
FROM dep_version INNER JOIN versions on dep_version.version_from=versions.id) 
SELECT COUNT(versions) FROM versions WHERE versions.crate_id<ANY(SELECT max FROM max_crate)
-- Temp backup
SELECT * FROM versions ORDER BY crate_id asc LIMIT 100 OFFSET 177000
-- Indirect Current resolved Crates_from counts
SELECT COUNT(DISTINCT crate_id) FROM dep_version INNER JOIN versions ON dep_version.version_from=versions.crate_id
-- Indirect Current resolved Version_from counts
SELECT COUNT(DISTINCT version_from) FROM dep_version 
-- Yanked versions that have dependencies
SELECT COUNT(DISTINCT versions.id) FROM dependencies INNER JOIN versions 
ON versions.id = dependencies.version_id WHERE versions.yanked = true
-- Select unresolved version including yanked version)
SELECT COUNT(*) FROM versions WHERE 
id NOT IN (SELECT DISTINCT version_from FROM dep_version) AND
id IN (SELECT DISTINCT version_id FROM dependencies) AND
id NOT IN (SELECT ver FROM dep_errors) ;
-- Find ver with no direct dependency
WITH dep_dis_ver AS
(SELECT DISTINCT version_from FROM dep_version)
SELECT * FROM dep_dis_ver WHERE version_from NOT IN (SELECT DISTINCT version_id FROM dependencies) LIMIT 100;
-- Find crates whose versions are all yanked
SELECT crate_id FROM versions WHERE crate_id NOT IN 
(SELECT DISTINCT crate_id FROM versions WHERE yanked = false);

-- Export DATABASE 
copy dep_version to 'version_dep.csv' WITH CSV DELIMITER ',';



-- Create Temporate Table for fast query

-- Create Indir Dependency crate->crate_id
CREATE TABLE dep_crate AS
WITH crate_from AS
(SELECT DISTINCT versions.crate_id as crate_from,  dep_version.version_to as version_to  
FROM dep_version INNER JOIN versions ON versions.id=dep_version.version_from)
SELECT DISTINCT crate_from.crate_from as crate_from,  versions.crate_id as crate_to  
FROM crate_from INNER JOIN versions ON versions.id=crate_from.version_to


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





-- Crate Version Overvew by year
with id_group as 
(SELECT COUNT(id)  as version_num_crate FROM versions WHERE created_at < '2015-01-01' GROUP BY crate_id )
SELECT COUNT(version_num_crate ),version_num_crate FROM id_group  GROUP BY version_num_crate ORDER BY version_num_crate  asc
-- TOP Crate with most versions
with id_group as 
(SELECT COUNT(id) as version_num_crate  , crate_id, SUM(downloads) as all_downloads FROM versions  GROUP BY crate_id )
SELECT version_num_crate, id_group.crate_id, crates.name, all_downloads    FROM id_group INNER JOIN crates ON id_group.crate_id=crates.id  ORDER BY id_group.version_num_crate  desc

-- TOP total downloads versions
SELECT versions.downloads as version_download, versions.id as version_id, crate_id, name 
FROM versions INNER JOIN crates ON versions.crate_id=crates.id 
ORDER BY version_download desc 
LIMIT 100
-- TOP total downloads crates
SELECT downloads, id ,name FROM crates ORDER BY downloads desc LIMIT 100

-- Hot Keywords by Crates
SELECT * FROM keywords ORDER BY crates_cnt desc LIMIT 100 
-- Hot Keywords by Downloads
with hot_keywords as 
(SELECT SUM(downloads) as total_downloads, keyword_id FROM crates_keywords INNER JOIN crates ON crate_id=id GROUP BY keyword_id )
SELECT keyword, total_downloads FROM hot_keywords INNER JOIN keywords ON keyword_id=id ORDER BY total_downloads desc LIMIT 100
-- Hot Category by Crates
SELECT * FROM categories ORDER BY crates_cnt desc LIMIT 100 
-- Hot Category by Downloads
with hot_categories  as 
(SELECT SUM(downloads) as total_downloads, category_id FROM crates_categories  INNER JOIN crates ON crate_id=id GROUP BY category_id )
SELECT category, total_downloads FROM hot_categories  INNER JOIN categories ON category_id=id ORDER BY total_downloads desc LIMIT 100

-- Owner: 78924/78935 crates have owner, 
-- -- and each crates may have multiple owners but only one creator.
-- Hot Owner by Crates
SELECT owner_id  ,COUNT(crate_id) as owned_count FROM crate_owners GROUP BY owner_id ORDER BY owned_count desc LIMIT 100
-- Hot Owner by Downloads
with hot_owner  as 
(SELECT SUM(downloads) as total_downloads, owner_id, COUNT(id) as count_crates FROM crate_owners INNER JOIN crates ON crate_id=id GROUP BY owner_id )
SELECT name, total_downloads, count_crates  , gh_login as GithubAccount, gh_avatar as GithubAvatar, gh_id as GithubID FROM hot_owner  INNER JOIN users ON owner_id =id ORDER BY total_downloads desc LIMIT 100
-- Hot Owner by Indir Dependents (Need to build table `dep_crate` first)
WITH hot_owner AS 
(SELECT owner_id, COUNT(DISTINCT crate_from) AS total_dependents FROM crate_owners INNER JOIN dep_crate ON crate_id=crate_to GROUP BY owner_id)
SELECT name, total_dependents, gh_login as GithubAccount, gh_avatar as GithubAvatar, gh_id as GithubID 
FROM hot_owner INNER JOIN users ON owner_id =id ORDER BY total_dependents desc LIMIT 100
-- Accumulative Hot Owner by Indir Dependents of TOP `N=50` (`N`<=50)
-- ATTENTION: It uses table `hot_owner`. 
DROP TABLE IF EXISTS tmp_owner_indir_crate,tmp_hot_owner_id,accumulate_hot_owners;
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

-- How many owners do hot crates have
SELECT name as crate_name, COUNT(owner_id) as owner_count, downloads FROM crates INNER JOIN crate_owners ON id=crate_id GROUP BY id ORDER BY downloads desc LIMIT 100
-- Top 500 (Downloads) owners
SELECT * FROM crate_owners WHERE crate_id IN (SELECT id FROM crates ORDER BY downloads desc LIMIT 500);


-- Recent Downloads

-- Version Recent 90days Downloads
SELECT version_id, SUM(downloads) as recent_downloads FROM version_downloads GROUP BY version_id ORDER BY recent_downloads desc LIMIT 100 
-- Crate Recent 90days Downloads
with version_recent as 
(SELECT version_id, SUM(downloads) as recent_downloads FROM version_downloads GROUP BY version_id )
SELECT crate_id, SUM(recent_downloads) as recent_downloads FROM version_recent INNER JOIN versions ON id=version_id GROUP BY crate_id ORDER BY recent_downloads desc LIMIT 100
-- Crate Recent 90days Downloads, full info
with crate_recent as(
with version_recent as 
(SELECT version_id, SUM(downloads) as recent_downloads FROM version_downloads GROUP BY version_id ORDER BY recent_downloads desc )
SELECT crate_id, SUM(recent_downloads) as recent_downloads FROM version_recent INNER JOIN versions ON id=version_id GROUP BY crate_id ORDER BY recent_downloads desc
) SELECT * FROM crate_recent INNER JOIN crates ON crates.id = crate_id ORDER BY recent_downloads desc LIMIT 100




-- Direct Dependency

-- How many versions have depdency (456156 before 20220315_015341)
SELECT COUNT(DISTINCT version_id) FROM dependencies

-- Version Direct Dependency Overvew
with ver_dep as 
(SELECT version_id, COUNT(id) as direct_dep FROM dependencies  GROUP BY version_id )
SELECT COUNT(version_id), direct_dep, to_char(created_at,'yyyy') as year 
FROM ver_dep INNER JOIN versions ON ver_dep.version_id=versions.id  
GROUP BY direct_dep, year
ORDER BY year desc, direct_dep desc

-- Version -> Crate
SELECT COUNT(DISTINCT version_id) FROM dependencies LIMIT 100
SELECT COUNT(DISTINCT crate_id) FROM dependencies LIMIT 100
-- Crate -> Crate
SELECT COUNT(DISTINCT versions.crate_id) FROM dependencies INNER JOIN versions ON versions.id=dependencies.version_id LIMIT 100
SELECT COUNT(DISTINCT crate_id) FROM dependencies LIMIT 100
-- Crate -> Crate (Not full yanked)

WITH dep_crate AS (SELECT DISTINCT versions.crate_id AS crate_id FROM dependencies 
				   INNER JOIN versions ON versions.id=dependencies.version_id )
SELECT COUNT(crate_id) FROM dep_crate WHERE crate_id NOT IN 
(SELECT crate_id FROM versions WHERE crate_id NOT IN 
(SELECT DISTINCT crate_id FROM versions WHERE yanked = false)) LIMIT 100

WITH dep_crate AS (SELECT DISTINCT crate_id AS crate_id FROM dependencies)
SELECT COUNT(crate_id) FROM dep_crate WHERE crate_id NOT IN 
(SELECT crate_id FROM versions WHERE crate_id NOT IN 
(SELECT DISTINCT crate_id FROM versions WHERE yanked = false)) LIMIT 100

-- Version -> Version
SELECT COUNT(DISTINCT version_from) FROM dep_version LIMIT 100
SELECT COUNT(DISTINCT version_to) FROM dep_version LIMIT 100

-- Top direct dep crates
WITH dep_crate AS(SELECT DISTINCT versions.crate_id as dep_from,  dependencies.crate_id as dep_to  
FROM dependencies INNER JOIN versions ON versions.id=dependencies.version_id  WHERE versions.yanked=false),
depcount_crate AS(SELECT dep_to, COUNT(*) as dependents FROM dep_crate GROUP BY dep_to)
SELECT name, dep_to, dependents FROM depcount_crate INNER JOIN crates ON dep_to=crates.id 
ORDER BY dependents desc LIMIT 100



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
DROP TABLE IF EXISTS tmp_hot_dep_crates,accumulate_hot_crates;
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

-- Advisory Propagation
SELECT COUNT(DISTINCT version_from) FROM dep_version WHERE version_to IN (SELECT * FROM advisory);
-- "=version" Propagation (rough)
SELECT COUNT(DISTINCT version_from) FROM dep_version WHERE version_to IN
(SELECT id  FROM dependencies WHERE req LIKE '=%' AND optional = false);
