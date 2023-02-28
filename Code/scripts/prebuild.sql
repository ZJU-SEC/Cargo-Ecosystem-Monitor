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
        (SELECT crate_id, MAX(created_at) as created_at FROM versions GROUP BY crate_id ORDER BY crate_id asc)
	SELECT newest_version.crate_id, id as newest_version_id, num as version_num, yanked FROM versions INNER JOIN newest_version
	ON versions.crate_id = newest_version.crate_id AND versions.created_at = newest_version.created_at ORDER BY crate_id asc)
SELECT crate_id, newest_version_id, name, version_num, yanked FROM crate_newest_version INNER JOIN crates ON crate_id = id);


-- Build Table (After execute projects `test_feature`)
CREATE TABLE feature_status AS
    SELECT name, v1_67_0 as status FROM "feature_timeline";
INSERT INTO feature_status
    SELECT DISTINCT feature, NULL AS status FROM version_feature_ori WHERE feature NOT IN (SELECT name FROM feature_status);

-- Build View: Version and crate info for every version
CREATE VIEW versions_with_name as (
        SELECT versions.*, crates.name FROM versions INNER JOIN crates ON versions.crate_id = crates.id);





-- Optional: Only executed when needed. Make sure you know what's going to happen.ADD
-- Re-resolve failed versions.
UPDATE deps_process_status SET status='undone' WHERE version_id IN (
    SELECT version_id FROM deps_process_status WHERE status='fail'
);
DROP TABLE dep_errors;