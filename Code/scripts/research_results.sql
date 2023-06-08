-- Result 1.1: RUF Status
SELECT count(*), v1_63_0 FROM feature_timeline GROUP BY v1_63_0;
-- Result 1.1 (Complete): RUF Status
SELECT name, v1_63_0 FROM feature_timeline ;

-- Result 1.2 (Complete): Abnormal RUF Lifetime.
SELECT feature_timeline.* FROM feature_abnormal INNER JOIN feature_timeline ON feature_timeline.name = feature_abnormal.name;

-- Result 2: RUF Usage
-- RUF count
SELECT status, COUNT(*) FROM feature_status
  WHERE name in (SELECT DISTINCT feature FROM version_feature_ori)
  GROUP BY status;
-- RUF count (Complete)
SELECT status, * FROM feature_status
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
  (SELECT id, status FROM version_feature_ori INNER JOIN feature_status
  ON name=feature WHERE conds = '' AND feature is not NULL)
SELECT DISTINCT version_from, status FROM uncon_ver INNER JOIN dep_version ON
version_to=id;
-- Cond Impact
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
-- Cond Impact (Complete)
SELECT DISTINCT id, status FROM tmp_ruf_impact;
-- Total Impact
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
-- Cond Impact (Complete)
SELECT DISTINCT id, status FROM tmp_ruf_impact;

-- Result 3.2 Super Spreaders
