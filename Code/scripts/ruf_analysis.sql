-- RUF propagation (Conditional)
SELECT  status, COUNT (DISTINCT(version_from)) FROM dep_version_feature 
INNER JOIN feature_status ON name=nightly_feature GROUP BY status;

-- RUF propagation (Unconditional)
WITH uncon_ver AS
  (SELECT id, status FROM version_feature_ori INNER JOIN feature_status
  ON name=feature WHERE conds = '' AND feature is not NULL)
SELECT status, COUNT(DISTINCT version_from) FROM uncon_ver INNER JOIN dep_version ON
version_to=id GROUP BY status;

-- RUF propagation (Simple Conditional)
WITH uncon_ver AS
  (SELECT id, status FROM version_feature INNER JOIN feature_status 
  ON name=feature WHERE conds = '' AND feature != 'no_feature_used')
SELECT status, COUNT(DISTINCT version_from) FROM uncon_ver INNER JOIN dep_version ON
version_to=id GROUP BY status;

-- RUF usage
SELECT  status, COUNT( DISTINCT id) FROM version_feature INNER JOIN feature_status 
ON name=feature WHERE feature != 'no_feature_used' GROUP BY status;

-- Total
DROP TABLE IF EXISTS tmp_ruf_impact;
CREATE TABLE tmp_ruf_impact AS (
    SELECT DISTINCT status, version_from as id FROM dep_version_feature 
    INNER JOIN feature_status ON name=nightly_feature 
);
WITH uncon_ver AS
  (SELECT id, status FROM version_feature INNER JOIN feature_status 
  ON name=feature WHERE conds = '' AND feature != 'no_feature_used')
INSERT INTO tmp_ruf_impact
SELECT DISTINCT status,  version_from FROM uncon_ver INNER JOIN dep_version ON version_to=id;
INSERT INTO tmp_ruf_impact
SELECT  DISTINCT status,  id FROM version_feature INNER JOIN feature_status 
ON name=feature WHERE feature != 'no_feature_used';
SELECT status, COUNT(DISTINCT id) FROM tmp_ruf_impact GROUP BY status;



-- Hot RUF (Indir dep)
DROP TABLE IF EXISTS tmp_hot_ruf_indir;
CREATE TABLE tmp_hot_ruf_indir AS (
    SELECT DISTINCT nightly_feature, version_from as id FROM dep_version_feature 
    INNER JOIN feature_status ON name=nightly_feature
);
WITH uncon_ver AS
  (SELECT DISTINCT id, feature FROM version_feature INNER JOIN feature_status 
  ON name=feature WHERE conds = '' AND feature != 'no_feature_used')
INSERT INTO tmp_hot_ruf_indir
SELECT DISTINCT feature, version_from FROM uncon_ver INNER JOIN dep_version ON version_to=id;
WITH hot_ruf AS(SELECT nightly_feature, COUNT(DISTINCT id) FROM tmp_hot_ruf_indir GROUP BY nightly_feature)
SELECT nightly_feature, count, status FROM hot_ruf INNER JOIN feature_status
ON  nightly_feature = name ORDER BY count DESC;



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