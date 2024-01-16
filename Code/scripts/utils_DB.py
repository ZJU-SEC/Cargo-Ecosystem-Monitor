import psycopg2

conn = psycopg2.connect(
    host="localhost",
    database="crates",
    user="postgres",
    password="postgres"
)

def get_earliest_crate_crated_time_DB():
    '''
    Return Earliest Crate Created Time `dict` from DB. <crate_id, created_at>
    '''
    cursor = conn.cursor()
    try:
        cursor.execute(f"SELECT MIN(created_at) FROM crates ")
        records = cursor.fetchall()
        return records[0][0]
    except Exception as e:
        print(e)
        return None


def get_crate_created_time_DB() -> dict:
    '''
    Return Crate Created Time `dict` from DB. <crate_id, created_at>
    '''
    cursor = conn.cursor()
    try:
        cursor.execute(f"SELECT id, created_at FROM crates ")
        records = cursor.fetchall()
        crates = dict()
        for record in records:
            crates[record[0]] = record[1]
        return crates
    except Exception as e:
        print(e)
        return None


def get_version_created_time_DB() -> dict:
    '''
    Return Version Created Time `dict` from DB. <version_id, created_at>
    '''
    cursor = conn.cursor()
    try:
        cursor.execute(f"SELECT id, created_at FROM versions")
        records = cursor.fetchall()
        versions = dict()
        for record in records:
            versions[record[0]] = record[1]
        return versions
    except Exception as e:
        print(e)
        return None


def get_crate_versionid_relation_DB() -> dict:
    '''
    Return Crate VersionID Relation `dict` from DB. <crate_id, [version_id]>
    '''
    cursor = conn.cursor()
    try:
        cursor.execute(f"SELECT id, crate_id FROM versions_with_name ORDER BY crate_id, id  ASC")
        records = cursor.fetchall()
        crates = dict()
        for record in records:
            crate_id = record[1]
            if crate_id not in crates:
                crates[crate_id] = list()
            crates[crate_id].append(record[0])
        return crates
    except Exception as e:
        print(e)
        return None



def get_ruf_lifetime_DB() -> dict:
    '''
    Return RUF Lifetime `dict` from DB. <ruf, [lifetime]>
    '''
    cursor = conn.cursor()
    try:
        cursor.execute(f"SELECT * FROM feature_timeline")
        records = cursor.fetchall()
        ruf_lifetime = dict()
        for record in records:
            ruf = record[0]
            lifetime = record[1:]
            ruf_lifetime[ruf] = lifetime
        return ruf_lifetime
    except Exception as e:
        print(e)
        return None


def get_ruf_usage_count_DB() -> dict:
    '''
    Return Usage Count `dict` from DB. <version_id, usage_count>
    Some versions are not included because we did not sucessfully get their ruf usage.
    '''
    cursor = conn.cursor()
    try:
        cursor.execute(f"SELECT id, feature FROM version_feature_ori ORDER BY id ASC")
        records = cursor.fetchall()
        usage_count = dict()
        for record in records:
            if record[0] not in usage_count:
                usage_count[record[0]] = 0
            if record[1] is not None:
                usage_count[record[0]] += 1
        return usage_count
    except Exception as e:
        print(e)
        return None
    

def get_version_recent_downloads_DB() -> dict:
    '''
    Return Version Recent Downloads `dict` from DB. <version_id, recent_downloads>
    '''
    cursor = conn.cursor()
    try:
        cursor.execute(f"SELECT version_id, SUM(downloads) FROM version_downloads GROUP BY version_id")
        records = cursor.fetchall()
        downloads = dict()
        for record in records:
            downloads[record[0]] = record[1]
        return downloads
    except Exception as e:
        print(e)
        return None
    

def get_crates_by_used_ruf_DB(ruf: str) -> list:
    '''
    Return Crates By Used RUF `list` from DB. [crate_id]
    '''
    cursor = conn.cursor()
    try:
        cursor.execute(f"SELECT DISTINCT crate_id FROM version_feature_ori INNER JOIN versions ON version_feature_ori.id = versions.id WHERE feature = '{ruf}'")
        records = cursor.fetchall()
        crates = list()
        for record in records:
            crates.append(record[0])
        return crates
    except Exception as e:
        print(e)
        return None



def get_used_ruf_by_versionid_DB_prebuild() -> dict:
    '''
    Return Used RUF By VersionID `dict` from DB. <version_id, [ruf]>
    '''
    cursor = conn.cursor()
    try:
        cursor.execute(f"SELECT id, feature FROM version_feature_ori WHERE feature IS NOT NULL ORDER BY id ASC")
        records = cursor.fetchall()
        rufs = dict()
        for record in records:
            version_id = record[0]
            if version_id not in rufs:
                rufs[version_id] = list()
            rufs[version_id].append(record[1])
        return rufs
    except Exception as e:
        print(e)
        return None


USED_RUF_BY_VERSIONID = get_used_ruf_by_versionid_DB_prebuild()
def get_used_ruf_by_versionid_DB(version_id: int) -> list:
    '''
    Return Used RUF By VersionID `list` from DB. [ruf]
    '''
    if version_id in USED_RUF_BY_VERSIONID:
        return USED_RUF_BY_VERSIONID[version_id]
    else:
        return list()


def get_crate_versions_created_time_asc_DB_prebuild() -> dict:
    '''
    Return Crate Versions Created Time `dict` from DB. <crate_id, [(version_id, created_at)]>
    '''
    cursor = conn.cursor()
    try:
        cursor.execute(f"SELECT id, crate_id, created_at FROM versions ORDER BY crate_id, created_at ASC")
        records = cursor.fetchall()
        crates = dict()
        for record in records:
            crate_id = record[1]
            if crate_id not in crates:
                crates[crate_id] = list()
            crates[crate_id].append((record[0], record[2]))
        return crates
    except Exception as e:
        print(e)
        return None


CRATE_VERSIONS_CREATED_TIME = get_crate_versions_created_time_asc_DB_prebuild()
def get_crate_versions_created_time_asc_DB(crate_id: int) -> list:
    '''
    Return Crate Versions Created Time `dict` from DB. [(version_id, created_at)]
    '''
    return CRATE_VERSIONS_CREATED_TIME[crate_id]