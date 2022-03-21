-- Sql scripts tools
-- Resolved max crate_id
SELECT MAX(crate_id) FROM dep_version INNER JOIN versions on dep_version.version_from=versions.id
-- Find Current Max resolved Offset
with max_crate as (SELECT MAX(crate_id) FROM dep_version INNER JOIN versions on dep_version.version_from=versions.id) SELECT COUNT(versions) FROM versions WHERE versions.crate_id<ANY(SELECT max FROM max_crate)
-- Temp backup
SELECT * FROM versions ORDER BY crate_id asc LIMIT 100 OFFSET 177000
