-- Nightly Feature Overview
-- 1.Sample
SELECT COUNT(*) FROM version_feature WHERE feature != 'no_feature_used';
SELECT COUNT(DISTINCT id) FROM version_feature;
-- How many versions have nightly features
SELECT COUNT(DISTINCT id) FROM version_feature WHERE feature != 'no_feature_used';
SELECT COUNT(DISTINCT crate_id) FROM version_feature INNER JOIN versions 
    ON version_feature.id = versions.id  WHERE feature != 'no_feature_used';
SELECT COUNT(DISTINCT id) FROM version_feature WHERE conds = '' AND feature != 'no_feature_used';
SELECT COUNT(DISTINCT crate_id) FROM version_feature INNER JOIN versions 
    ON version_feature.id = versions.id  WHERE conds = '' AND feature != 'no_feature_used';
SELECT COUNT(DISTINCT id) FROM version_feature WHERE conds LIKE 'feature = %' AND feature != 'no_feature_used';
SELECT COUNT(DISTINCT crate_id) FROM version_feature INNER JOIN versions 
    ON version_feature.id = versions.id  WHERE conds LIKE 'feature = %' AND feature != 'no_feature_used';
-- Other unresolved features
SELECT * FROM version_feature WHERE conds like 'all(%'  LIMIT 100      ;

-- 2. Type
-- Feature items by type
SELECT id, feature, status FROM version_feature INNER JOIN feature_status ON feature = name;
SELECT COUNT(id), status FROM version_feature INNER JOIN feature_status ON feature = name GROUP BY status;
SELECT COUNT(DISTINCT id), status FROM version_feature INNER JOIN feature_status ON feature = name GROUP BY status;
WITH ruf_ver AS (
  SELECT id, status FROM version_feature INNER JOIN feature_status ON feature = name 
  )
SELECT COUNT(DISTINCT crate_id), status FROM ruf_ver INNER JOIN versions
    ON ruf_ver.id=versions.id GROUP BY status;


-- 3. Top
-- Hot features by usage
WITH hot_feat AS(
    SELECT feature, COUNT(*) FROM version_feature WHERE feature != 'no_feature_used' GROUP BY feature ORDER BY count DESC
) SELECT feature, count, status FROM hot_feat INNER JOIN feature_status ON feature = name;



-- Nightly Feature Propagation
-- 1. Unconditional nightly features
-- Indir dependents
SELECT COUNT(DISTINCT version_from) FROM dep_version WHERE version_to IN(
    SELECT id FROM version_feature WHERE conds = '' AND feature != 'no_feature_used'
);
WITH uncon_ver AS (
    SELECT DISTINCT version_from FROM dep_version WHERE version_to IN(
        SELECT id FROM version_feature WHERE conds = '' AND feature != 'no_feature_used'
))
SELECT COUNT(DISTINCT crate_id) FROM uncon_ver INNER JOIN versions
    ON version_from=id;
-- How many versions/crates have indir dependents
SELECT COUNT(DISTINCT version_to) FROM dep_version WHERE version_to IN(
    SELECT id FROM version_feature WHERE conds = '' AND feature != 'no_feature_used'
);
WITH uncon_ver AS (
    SELECT DISTINCT version_to FROM dep_version WHERE version_to IN(
        SELECT id FROM version_feature WHERE conds = '' AND feature != 'no_feature_used'
))
SELECT COUNT(DISTINCT crate_id) FROM uncon_ver INNER JOIN versions
    ON version_to=id;
-- Hot version(RUF) with most dependents
WITH uncon_ver_sta AS (
	WITH uncon_ver AS (
		SELECT versions_with_name.id, name, num, feature FROM version_feature_ori INNER JOIN versions_with_name
			ON version_feature_ori.id = versions_with_name.id WHERE conds = '' AND feature is not NULL)
	SELECT id, uncon_ver.name, num, feature, status FROM uncon_ver INNER JOIN feature_status 
		ON uncon_ver.feature = feature_status.name)
, indir_dep AS (
	SELECT version_to, COUNT(DISTINCT version_from) FROM dep_version WHERE version_to IN(
    SELECT id FROM version_feature_ori WHERE conds = '' AND feature is not NULL
	) GROUP BY version_to)
SELECT uncon_ver_sta.name, num, count AS dependents, feature, status
FROM uncon_ver_sta INNER JOIN indir_dep ON id=version_to
ORDER BY dependents DESC


-- 2. Conditional nightly features
-- Possible indir dependents
SELECT COUNT(DISTINCT version_from) FROM dep_version WHERE version_to IN(
    SELECT id FROM version_feature WHERE conds LIKE 'feature = %' AND feature != 'no_feature_used'
);

-- How many versions have indir dependents
SELECT COUNT(DISTINCT version_to) FROM dep_version WHERE version_to IN(
    SELECT id FROM version_feature WHERE conds LIKE 'feature = %' AND feature != 'no_feature_used'
);
-- Indir dependents
SELECT COUNT(DISTINCT version_from) FROM dep_version_feature;
SELECT COUNT(DISTINCT crate_id) 
FROM dep_version_feature INNER JOIN versions ON version_from = id;

-- 3. Total nightly features
SELECT COUNT(DISTINCT version_from) 
FROM dep_version_feature WHERE version_from NOT IN (
    SELECT DISTINCT version_from FROM dep_version WHERE version_to IN(
        SELECT id FROM version_feature WHERE conds = '' AND feature != 'no_feature_used')
);
SELECT COUNT(DISTINCT crate_id) 
FROM dep_version_feature INNER JOIN versions ON version_from = id
WHERE crate_id NOT IN (
    WITH uncon_ver AS (
    SELECT DISTINCT version_to FROM dep_version WHERE version_to IN(
        SELECT id FROM version_feature WHERE conds = '' AND feature != 'no_feature_used'
    ))
    SELECT DISTINCT crate_id FROM uncon_ver INNER JOIN versions
        ON version_to=id
);


-- 3. Different types of nightly features
SELECT status, COUNT((status)) FROM feature_status GROUP BY status ;
SELECT  status, COUNT (DISTINCT(version_from)) FROM dep_version_feature 
INNER JOIN feature_status ON name=nightly_feature GROUP BY status ;
SELECT COUNT(DISTINCT version_from) FROM dep_version WHERE version_to IN(
    SELECT id FROM version_feature INNER JOIN feature_status 
    ON name=feature WHERE conds = '' AND feature != 'no_feature_used' AND status = 'xxx'
);
-- Top dependents
WITH deps AS (
    SELECT version_to, COUNT(DISTINCT version_from) as dependents
    FROM dep_version
    WHERE version_to IN(
        SELECT id FROM version_feature INNER JOIN feature_status 
        ON name=feature WHERE conds = '' AND feature != 'no_feature_used' AND status = 'xxx'
    ) GROUP BY version_to
)
SELECT deps.*, crate_id, num ,name
FROM deps INNER JOIN versions_with_name ON id=version_to ORDER BY dependents DESC;




-- RUF Evolution

SELECT versions.id, feature, created_at FROM version_feature INNER JOIN versions
ON version_feature.id = versions.id WHERE feature != 'no_feature_used' AND created_at < '2022-06-01' ORDER BY created_at DESC;

DROP TABLE IF EXISTS tmp_year_month;
CREATE TABLE tmp_year_month(
    crate_date date
);
INSERT INTO tmp_year_month VALUES
    ('2015-01-01'), ('2015-04-01'), ('2015-07-01'), ('2015-10-01'),
    ('2016-01-01'), ('2016-04-01'), ('2016-07-01'), ('2016-10-01'),
    ('2017-01-01'), ('2017-04-01'), ('2017-07-01'), ('2017-10-01'),
    ('2018-01-01'), ('2018-04-01'), ('2018-07-01'), ('2018-10-01'),
    ('2019-01-01'), ('2019-04-01'), ('2019-07-01'), ('2019-10-01'),
    ('2020-01-01'), ('2020-04-01'), ('2020-07-01'), ('2020-10-01'),
    ('2021-01-01'), ('2021-04-01'), ('2021-07-01'), ('2021-10-01'),
    ('2022-01-01'), ('2022-04-01'), ('2022-07-01'), ('2022-08-12')
;

-- Version (ruf)
WITH all_ruf AS(
  SELECT versions.id, feature, created_at FROM version_feature INNER JOIN versions
ON version_feature.id = versions.id WHERE feature != 'no_feature_used'
) 
SELECT crate_date, COUNT(DISTINCT id) FROM all_ruf INNER JOIN tmp_year_month
ON crate_date > created_at GROUP BY crate_date ORDER BY crate_date ASC;

-- Crate (ruf)
WITH all_ruf AS(
  SELECT versions_with_name.crate_id, feature, created_at FROM version_feature INNER JOIN versions_with_name
ON version_feature.id = versions_with_name.id WHERE feature != 'no_feature_used'
) 
SELECT crate_date, COUNT(DISTINCT crate_id) FROM all_ruf INNER JOIN tmp_year_month
ON crate_date > created_at GROUP BY crate_date ORDER BY crate_date ASC;

-- Version (all)

SELECT crate_date, COUNT(DISTINCT id) FROM versions INNER JOIN tmp_year_month
ON crate_date > created_at GROUP BY crate_date ORDER BY crate_date ASC;