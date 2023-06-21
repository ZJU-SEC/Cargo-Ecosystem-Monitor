-- 1. RUF lifetime (status)

SELECT COUNT(*) FROM feature_timeline;
SELECT status, COUNT(*) FROM feature_status GROUP BY status;


-- 2. RUF Usage
-- Used RUF
SELECT COUNT(*) FROM feature_status
  WHERE name in (SELECT DISTINCT feature FROM version_feature_ori);
SELECT status, COUNT(*) FROM feature_status
  WHERE name in (SELECT DISTINCT feature FROM version_feature_ori)
  GROUP BY status;
-- Versions that use RUF
SELECT COUNT(DISTINCT id) FROM version_feature_ori INNER JOIN feature_status ON name=feature; 
SELECT status, COUNT(DISTINCT id) FROM version_feature_ori INNER JOIN feature_status ON name=feature GROUP BY status;
SELECT COUNT(DISTINCT id) FROM version_feature_ori INNER JOIN feature_status ON name=feature WHERE status != 'accepted';  
SELECT COUNT(DISTINCT id) FROM version_feature_ori INNER JOIN feature_status ON name=feature WHERE status = 'removed' OR status IS NULL;  
-- RUF Usage Items
SELECT COUNT(*) FROM version_feature_ori INNER JOIN feature_status ON name=feature; 
SELECT status, COUNT(*) FROM version_feature_ori INNER JOIN feature_status ON name=feature GROUP BY status; 


-- 3. RUF Impacts
-- Direct Impact
SELECT COUNT(*) FROM feature_status
  WHERE name in (SELECT DISTINCT feature FROM version_feature_ori);
SELECT status, COUNT(*) FROM feature_status
  WHERE name in (SELECT DISTINCT feature FROM version_feature_ori)
  GROUP BY status;
-- Unconditional Impact
WITH uncon_ver AS
  (SELECT id, status FROM version_feature_ori INNER JOIN feature_status
  ON name=feature WHERE conds = '' AND feature is not NULL)
SELECT COUNT(DISTINCT version_from) FROM uncon_ver INNER JOIN dep_version ON
version_to=id;
WITH uncon_ver AS
  (SELECT id, status FROM version_feature_ori INNER JOIN feature_status
  ON name=feature WHERE conds = '' AND feature is not NULL)
SELECT status, COUNT(DISTINCT version_from) FROM uncon_ver INNER JOIN dep_version ON
version_to=id GROUP BY status;
-- Simple Conditional Impact: Includes Uncond and other easily identified impacts.
WITH uncon_ver AS
  (SELECT id, status FROM version_feature INNER JOIN feature_status 
  ON name=feature WHERE conds = '' AND feature is not NULL)
SELECT COUNT(DISTINCT version_from) FROM uncon_ver INNER JOIN dep_version ON
version_to=id;
WITH uncon_ver AS
  (SELECT id, status FROM version_feature INNER JOIN feature_status 
  ON name=feature WHERE conds = '' AND feature is not NULL)
SELECT status, COUNT(DISTINCT version_from) FROM uncon_ver INNER JOIN dep_version ON
version_to=id GROUP BY status;
-- Indirect Impact (Associate with package feature)
SELECT  status, COUNT (DISTINCT(version_from)) FROM dep_version_feature 
INNER JOIN feature_status ON name=nightly_feature GROUP BY status;
-- Conditional Impact : SimpleCond+IndirImpact
DROP TABLE IF EXISTS tmp_ruf_impact;
CREATE TABLE tmp_ruf_impact AS (
    SELECT DISTINCT status, version_from as id FROM dep_version_feature 
    INNER JOIN feature_status ON name=nightly_feature 
);
WITH uncon_ver AS
  (SELECT id, status FROM version_feature INNER JOIN feature_status 
  ON name=feature WHERE conds = '' AND feature IS NOT NULL)
INSERT INTO tmp_ruf_impact
  SELECT DISTINCT status,  version_from FROM uncon_ver INNER JOIN dep_version ON version_to=id;
SELECT status, COUNT(DISTINCT id) FROM tmp_ruf_impact GROUP BY status;
SELECT COUNT(DISTINCT id) FROM tmp_ruf_impact;
-- Total Impact: Direct+Cond
DROP TABLE IF EXISTS tmp_ruf_impact;
CREATE TABLE tmp_ruf_impact AS (
    SELECT DISTINCT status, version_from as id FROM dep_version_feature 
    INNER JOIN feature_status ON name=nightly_feature 
);
WITH uncon_ver AS
  (SELECT id, status FROM version_feature INNER JOIN feature_status 
  ON name=feature WHERE conds = '' AND feature IS NOT NULL)
INSERT INTO tmp_ruf_impact
  SELECT DISTINCT status,  version_from FROM uncon_ver INNER JOIN dep_version ON version_to=id;
INSERT INTO tmp_ruf_impact
  SELECT  DISTINCT status,  id FROM version_feature INNER JOIN feature_status 
  ON name=feature WHERE feature IS NOT NULL;
SELECT status, COUNT(DISTINCT id) FROM tmp_ruf_impact GROUP BY status;
SELECT COUNT(DISTINCT id) FROM tmp_ruf_impact;
SELECT COUNT(DISTINCT id) FROM tmp_ruf_impact WHERE status = 'removed' OR status IS NULL; -- Extra: In Total Impact, sum of Unknown and Removed RUF Impact.


-- 4. Hot version using RUF (using certain status of RUF)
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
-- Hot RUF by usage
WITH hot_feat AS(
    SELECT feature, COUNT(*) FROM version_feature WHERE feature IS NOT NULL GROUP BY feature 
) SELECT feature, count, status FROM hot_feat INNER JOIN feature_status ON feature = name ORDER BY count DESC;
-- Abnormal Hot RUF by usage
WITH hot_feat AS(
    SELECT feature, COUNT(*) FROM version_feature WHERE feature IS NOT NULL GROUP BY feature 
) SELECT feature, count, status FROM hot_feat INNER JOIN feature_status ON feature = name 
WHERE feature IN (SELECT * FROM feature_abnormal)
ORDER BY count DESC;
-- Hot RUF by dependents
WITH hot_feat AS(
    SELECT feature, COUNT(DISTINCT version_from) FROM version_feature INNER JOIN dep_version ON id=version_to
    WHERE conds = '' GROUP BY feature 
) SELECT feature, count, status FROM hot_feat INNER JOIN feature_status ON feature = name ORDER BY count DESC;
-- Abnormal Hot RUF by dependents
WITH hot_feat AS(
    SELECT feature, COUNT(DISTINCT version_from) FROM version_feature INNER JOIN dep_version ON id=version_to
    WHERE conds = '' GROUP BY feature 
) SELECT feature, count, status FROM hot_feat INNER JOIN feature_status ON feature = name
WHERE feature IN (SELECT * FROM feature_abnormal)
ORDER BY count DESC;



-- 5. Hot version(RUF) with most dependents
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



-- 6. RUF Evolution: Only considers direct usage rather than indirect impacts.ADD
-- The results show the proportion of versions using RUF is declining, while we found that impacts are rising under limited study.

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



-- 7. RUF Remediation Analysis
-- In project "RUF Remediation Analysis", create RUF impact table <version, RUF>.
DROP TABLE IF EXISTS tmp_ruf_remediation_analysis;
CREATE TABLE tmp_ruf_remediation_analysis AS (
    SELECT DISTINCT id, feature FROM version_feature
    WHERE feature != 'no_feature_used'
);
INSERT INTO tmp_ruf_remediation_analysis
    SELECT DISTINCT version_from, feature FROM version_feature 
    INNER JOIN dep_version ON version_to=id WHERE conds = '' AND feature IS NOT NULL;
INSERT INTO tmp_ruf_remediation_analysis
    SELECT  DISTINCT version_from, nightly_feature FROM dep_version_feature;