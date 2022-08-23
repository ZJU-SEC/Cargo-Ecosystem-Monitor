-- Sql scripts tools
-- Resolved max crate_id
SELECT MAX(crate_id) FROM dep_version INNER JOIN versions on dep_version.version_from=versions.id

-- Find Current Max resolved Offset
with max_crate as (SELECT MAX(crate_id) 
FROM dep_version INNER JOIN versions on dep_version.version_from=versions.id) 
SELECT COUNT(versions) FROM versions WHERE versions.crate_id<ANY(SELECT max FROM max_crate)

-- Find current unresolved crate versions (Takes long time)
-- Dependencies(Without dev-dep) - yanked - dep_errors - dep_version
WITH ver_feature AS
    (SELECT id as version_id, crate_id, num FROM versions WHERE id in 
        (WITH ver_dep AS
            (SELECT DISTINCT version_id as ver FROM dependencies WHERE kind != 2)
        SELECT ver FROM ver_dep
        WHERE ver NOT IN (SELECT id FROM versions WHERE yanked = true) 
        AND ver NOT IN (SELECT DISTINCT ver FROM dep_errors)
        AND ver NOT IN (SELECT DISTINCT version_from FROM dep_version))
    )
SELECT version_id, crate_id, name, num FROM crates INNER JOIN ver_feature ON crate_id=id
-- Build this table
CREATE TABLE tmp_cached_ver_feature AS (...)


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
copy dep_version to 'path/to/version_dep_20xxxxxx.csv' WITH CSV DELIMITER ',';
-- Import from csv
COPY dep_version(version_from, version_to, dep_level) FROM 'path/to/version_dep_20xxxxxx.csv' DELIMITER ',' CSV HEADER;


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
-- Top 500 (Downloads) owners
SELECT * FROM crate_owners WHERE crate_id IN (SELECT id FROM crates ORDER BY downloads desc LIMIT 500);
