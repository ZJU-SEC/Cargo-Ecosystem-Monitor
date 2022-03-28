-- Sql scripts tools
-- Resolved max crate_id
SELECT MAX(crate_id) FROM dep_version INNER JOIN versions on dep_version.version_from=versions.id
-- Find Current Max resolved Offset
with max_crate as (SELECT MAX(crate_id) FROM dep_version INNER JOIN versions on dep_version.version_from=versions.id) 
SELECT COUNT(versions) FROM versions WHERE versions.crate_id<ANY(SELECT max FROM max_crate)
-- Temp backup
SELECT * FROM versions ORDER BY crate_id asc LIMIT 100 OFFSET 177000
-- Indirect Current resolved Crates_from counts
SELECT COUNT(DISTINCT crate_id) FROM dep_version INNER JOIN versions ON dep_version.version_from=versions.crate_id
-- Indirect Current resolved Crates_from counts
SELECT COUNT(DISTINCT version_from) FROM dep_version 

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
-- How many owners do hot crates have
SELECT name as crate_name, COUNT(owner_id) as owner_count, downloads FROM crates INNER JOIN crate_owners ON id=crate_id GROUP BY id ORDER BY downloads desc LIMIT 100




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
-- Version with most indirect dependency
SELECT version_from, COUNT(DISTINCT version_to) as indirect_dep FROM dep_version GROUP BY version_from ORDER BY indirect_dep desc LIMIT 100
-- Version with most indirect dependents
SELECT version_to, COUNT(DISTINCT version_from) as indirect_dep FROM dep_version GROUP BY version_to ORDER BY indirect_dep desc LIMIT 100
-- Version with most indirect dependents, full info
with indirect_deps AS(
SELECT version_to, COUNT(DISTINCT version_from) as indirect_dep FROM dep_version GROUP BY version_to 
),version_name AS (
SELECT name,versions.id as version_id  FROM versions INNER JOIN crates ON crates.id = versions.crate_id )
SELECT name, indirect_dep FROM version_name INNER JOIN indirect_deps ON indirect_deps.version_to = version_name.version_id 
ORDER BY indirect_dep desc LIMIT 100
-- Crate with most indirect dependents, full info
WITH crate_from AS(SELECT DISTINCT versions.crate_id as crate_from,  dep_version.version_to as version_to  
FROM dep_version INNER JOIN versions ON versions.id=dep_version.version_from),
crate_to AS(SELECT DISTINCT crate_from.crate_from as crate_from,  versions.crate_id as crate_to  
FROM crate_from INNER JOIN versions ON versions.id=crate_from.version_to),
indir_crate AS (SELECT crate_to, COUNT(*) as crate_dependents FROM crate_to 
GROUP BY crate_to ORDER BY crate_dependents)
SELECT name, crate_dependents FROM crates INNER JOIN indir_crate ON crates.id = indir_crate.crate_to 
ORDER BY crate_dependents desc LIMIT 100