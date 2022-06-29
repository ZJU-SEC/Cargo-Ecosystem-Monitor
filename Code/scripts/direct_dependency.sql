
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

