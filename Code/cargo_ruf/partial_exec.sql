CREATE TABLE IF NOT EXISTS ruf_audit_process_status
            (
                version_id INT,
                status VARCHAR
            );
INSERT INTO ruf_audit_process_status
SELECT DISTINCT ver, 'undone' FROM tmp_ruf_impact
WHERE status = 'removed' or status = 'unknown' OFFSET 40000 LIMIT 20000;

-- Add the rest
INSERT INTO ruf_audit_process_status 
SELECT DISTINCT ver, 'undone' FROM tmp_ruf_impact
WHERE 
    (status = 'removed' OR status = 'unknown') AND
    ver NOT IN (SELECT version_id FROM ruf_audit_process_status);