-- Create Temporate Table for fast query

-- Create Indir Dependency crate->crate_id
DROP TABLE IF EXISTS dep_crate;
CREATE TABLE dep_crate AS
WITH crate_from AS
(SELECT DISTINCT versions.crate_id as crate_from,  dep_version.version_to as version_to  
FROM dep_version INNER JOIN versions ON versions.id=dep_version.version_from)
SELECT DISTINCT crate_from.crate_from as crate_from,  versions.crate_id as crate_to  
FROM crate_from INNER JOIN versions ON versions.id=crate_from.version_to

-- Build Table: Find relation between crate and newest_version
DROP TABLE IF EXISTS crate_newestversion;
CREATE TABLE crate_newestversion AS
(
WITH crate_newest_version AS
    (WITH newest_version AS
        (SELECT crate_id, MAX(updated_at) as updated_at FROM versions GROUP BY crate_id ORDER BY crate_id asc)
	SELECT newest_version.crate_id, id as newest_version_id, num as version_num FROM versions INNER JOIN newest_version
	ON versions.crate_id = newest_version.crate_id AND versions.updated_at = newest_version.updated_at ORDER BY crate_id asc)
SELECT crate_id, newest_version_id, name, version_num FROM crate_newest_version INNER JOIN crates ON crate_id = id);