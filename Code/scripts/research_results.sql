-- Result 1.1: RUF Status
SELECT count(*), v1_63_0 as status FROM feature_timeline GROUP BY v1_63_0;
-- Result 1.1 (Complete): RUF Status
SELECT name, v1_63_0 as status FROM feature_timeline ;

-- Result 1.2 (Complete): Abnormal RUF Lifetime.
SELECT feature_timeline.* FROM feature_abnormal INNER JOIN feature_timeline ON feature_timeline.name = feature_abnormal.name;

-- Result 2: RUF Usage
-- RUF count
SELECT status, COUNT(*) FROM feature_status
  WHERE name in (SELECT DISTINCT feature FROM version_feature_ori)
  GROUP BY status;
-- RUF count (Complete)
SELECT name, status FROM feature_status
  WHERE name in (SELECT DISTINCT feature FROM version_feature_ori);
-- Package Versions
SELECT status, COUNT(DISTINCT id) FROM version_feature_ori INNER JOIN feature_status ON name=feature GROUP BY status;
-- RUF Usage Items
SELECT status, COUNT(*) FROM version_feature_ori INNER JOIN feature_status ON name=feature GROUP BY status; 
-- Package Versions + RUF Usage Items (Complete)
SELECT version_feature_ori.*, status FROM version_feature_ori INNER JOIN feature_status ON name=feature;

-- Result 3.1 RUF Impact
-- Direct Usage (See Result 2: RUF Usage)
-- Uncond Impact
WITH uncon_ver AS
  (SELECT id, status FROM version_feature_ori INNER JOIN feature_status
  ON name=feature WHERE conds = '' AND feature is not NULL)
SELECT status, COUNT(DISTINCT version_from) FROM uncon_ver INNER JOIN dep_version ON
version_to=id GROUP BY status;
-- Uncond Impact (Complete)
WITH uncon_ver AS
  (SELECT id, name as ruf, status FROM version_feature_ori INNER JOIN feature_status
  ON name=feature WHERE conds = '' AND feature is not NULL)
SELECT DISTINCT version_from, ruf, status FROM uncon_ver INNER JOIN dep_version ON
version_to=id;

-- Cond Impact
DROP TABLE IF EXISTS tmp_ruf_impact;
CREATE TABLE tmp_ruf_impact AS (
    SELECT DISTINCT version_from as ver, nightly_feature as ruf, status FROM dep_version_feature 
    INNER JOIN feature_status ON name=nightly_feature 
);
WITH uncon_ver AS
  (SELECT id, name as ruf, status FROM version_feature INNER JOIN feature_status
  ON name=feature WHERE conds = '' AND feature is not NULL)
INSERT INTO tmp_ruf_impact
  SELECT DISTINCT version_from, ruf, status FROM uncon_ver INNER JOIN dep_version ON
  version_to=id;
SELECT status, COUNT(DISTINCT ver) FROM tmp_ruf_impact GROUP BY status;
-- Cond Impact (Complete)
SELECT DISTINCT * FROM tmp_ruf_impact;

-- Total Impact
DROP TABLE IF EXISTS tmp_ruf_impact;
CREATE TABLE tmp_ruf_impact AS (
    SELECT version_from as ver, nightly_feature as ruf, status FROM dep_version_feature 
    INNER JOIN feature_status ON name=nightly_feature 
);
WITH uncon_ver AS
  (SELECT id, name as ruf, status FROM version_feature INNER JOIN feature_status
  ON name=feature WHERE conds = '' AND feature is not NULL)
INSERT INTO tmp_ruf_impact
  SELECT DISTINCT version_from, ruf, status FROM uncon_ver INNER JOIN dep_version ON
  version_to=id;
INSERT INTO tmp_ruf_impact
  SELECT  DISTINCT id, feature, status FROM version_feature INNER JOIN feature_status 
  ON name=feature WHERE feature IS NOT NULL;
SELECT status, COUNT(DISTINCT ver) FROM tmp_ruf_impact GROUP BY status;
-- Cond Impact (Complete)
SELECT DISTINCT * FROM tmp_ruf_impact;

-- Result 3.2 Super Spreaders
WITH deps AS (
    SELECT version_to, COUNT(DISTINCT version_from) as dependents
    FROM dep_version
    WHERE version_to IN(
        SELECT id FROM version_feature INNER JOIN feature_status 
        ON name=feature WHERE conds = '' AND feature != 'no_feature_used' AND status = 'unknown'
    ) GROUP BY version_to
)
SELECT name, num as version_num, dependents
FROM deps INNER JOIN versions_with_name ON id=version_to ORDER BY dependents DESC;