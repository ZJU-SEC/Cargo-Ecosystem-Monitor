-- RUF Definition
-- 1. All RUF
SELECT COUNT(*) FROM feature_timeline;
-- 2. RUF at last
SELECT status, COUNT(*) FROM feature_status GROUP BY status;



-- RUF Overview
-- 1.Sample
SELECT COUNT(*) FROM version_feature WHERE feature != 'no_feature_used';
SELECT COUNT(DISTINCT id) FROM version_feature;
-- How many versions have RUF
SELECT COUNT(DISTINCT id) FROM version_feature WHERE feature != 'no_feature_used';
SELECT COUNT(DISTINCT crate_id) FROM version_feature INNER JOIN versions 
    ON version_feature.id = versions.id  WHERE feature != 'no_feature_used';
SELECT COUNT(DISTINCT id) FROM version_feature WHERE conds = '' AND feature != 'no_feature_used';
SELECT COUNT(DISTINCT crate_id) FROM version_feature INNER JOIN versions 
    ON version_feature.id = versions.id  WHERE conds = '' AND feature != 'no_feature_used';
SELECT COUNT(DISTINCT id) FROM version_feature WHERE conds LIKE 'feature = %' AND feature != 'no_feature_used';
SELECT COUNT(DISTINCT crate_id) FROM version_feature INNER JOIN versions 
    ON version_feature.id = versions.id  WHERE conds LIKE 'feature = %' AND feature != 'no_feature_used';
-- Other unresolved RUF
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




-- RUF Impacts/Propagation/IndirectDependents
-- 1. Unconditional RUF
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
-- Hot version using RUF with most dependents
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


-- 2. Conditional RUF
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

-- 3. Total RUF
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


-- 3. Different types of RUF
SELECT status, COUNT((status)) FROM feature_status GROUP BY status ;
SELECT  status, COUNT (DISTINCT(version_from)) FROM dep_version_feature 
INNER JOIN feature_status ON name=nightly_feature GROUP BY status ;
SELECT COUNT(DISTINCT version_from) FROM dep_version WHERE version_to IN(
    SELECT id FROM version_feature INNER JOIN feature_status 
    ON name=feature WHERE conds = '' AND feature != 'no_feature_used' AND status = 'xxx'
);


