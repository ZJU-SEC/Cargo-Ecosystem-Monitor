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


-- Crates updated in a year

SELECT COUNT(DISTINCT(crate_id)) FROM versions WHERE created_at > '2021-03-01' LIMIT 100;
SELECT COUNT(DISTINCT(crate_id)) FROM versions WHERE created_at > '2020-03-01' LIMIT 100;
SELECT COUNT(DISTINCT(crate_id)) FROM versions WHERE created_at > '2019-03-01' LIMIT 100;
SELECT COUNT(DISTINCT(crate_id)) FROM versions WHERE created_at > '2018-03-01' LIMIT 100;
SELECT COUNT(DISTINCT(crate_id)) FROM versions WHERE created_at > '2017-03-01' LIMIT 100;