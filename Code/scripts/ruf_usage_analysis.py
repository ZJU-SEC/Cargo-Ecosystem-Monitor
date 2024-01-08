import re
import psycopg2

conn = psycopg2.connect(
    host="localhost",
    database="crates",
    user="postgres",
    password="postgres"
)

def get_version_date() -> dict:
    version_date = dict()
    r = requests.get('https://raw.githubusercontent.com/rust-lang/rust/master/RELEASES.md')
    text = r.text
    versions = re.findall("Version 1\.[0-9]+\.0 \([0-9]+-[0-9]+-[0-9]+\)", text)
    # `version` example: "Version 0.1  (2012-01-20)"
    print("Starting downloading rustdoc from " + versions[-2] + " to " + versions[0])
    for version in reversed(versions):
        version_list = version.split(' ')
        version_num = version_list[1]
        version_date = version_list[2].strip('(').strip(')')
        version_date[version_num] = version_date
    return version_date


def ruf_usage_lifetime():
    '''
    Main function.
    We mainly analyze how RUF is used by developers and how they maintain the RUF.
    0. (Overview) How many packages use RUF? Are they using more through time? What about packages with large impacts (downloads).
        We should not only focus on old crates releasing new versions, but also new crates seperately. This will reveal the true new usage.
        Differentiate new crates and new versions.
    1. (RUF Change -> Usage Change) RUF can be removed. How do they react to RUF removal? Do they update their code?
        RUF can be stabilized. Removing RUF usage can prevent nightly compiler selection.
    2. (Usage Change) Will developers try to actively remove RUF usage actively rather than after RUF removal?
        If so, how do they do it? Do they remove RUF usage only when RUF are stabilized or removed?
    3. RUF fix. Will developers backport their fix to old versions? Fix with minor/major version update may not apply to old versions.
        How many versions are still suffer? This is the critical point in semver-based ecosytem. 
    '''
    # 0. Overview
    ruf_usage_popular_crates()
    ruf_usage_intra_crate_evolution()
    ruf_usage_inter_crate_evolution()

    # 1. (RUF Change -> Usage Change)
    



def timedelta_in_months(end, start):                                # defining the function
    return 12 * (end.year - start.year) + (end.month - start.month) # returning the calculation


def ruf_usage_popular_crates():
    '''
    Compare RUF usage of popular and unpopular crates, using recent 3months download as index.
    Popular Level 5000000: 87 crates, 43 ruf, 0.4942528735632184 average
    Popular Level 50000: 5431 crates, 2209 ruf, 0.40673909040692324 average
    Popular Level 500: 48583 crates, 17241 ruf, 0.3548772204268983 average
    Popular Level 5: 538070 crates, 162532 ruf, 0.3020647871094839 average
    '''
    POPULAR_LEVELS = [5000000, 50000, 500, 5]
    recent_downloads = get_version_recent_downloads_DB()
    usage_count = get_ruf_usage_count_DB()
    popular_results = list()
    for level in POPULAR_LEVELS:
        popular_results.append([0, 0])
    for version in recent_downloads:
        downloads = recent_downloads[version]
        ruf_count = 0
        if version in usage_count:
            ruf_count = usage_count[version]
        for i in range(len(POPULAR_LEVELS)):
            if downloads >= POPULAR_LEVELS[i]:
                popular_results[i][0] += 1
                popular_results[i][1] += ruf_count
                break
    for i in range(len(POPULAR_LEVELS)):
        count = popular_results[i][0]
        ruf_count = popular_results[i][1]
        average = ruf_count / count
        print(f"Popular Level {POPULAR_LEVELS[i]}: {count} crates, {ruf_count} ruf, {average} average")


        



def ruf_usage_intra_crate_evolution():
    '''
    RUF Usage Evolution: Looked into each crate and see versions RUF usage evolution.
    Will a single package use more ruf through time?
    Quite the same. In average, one crate uses 0.2 ruf, whenever it is old or new.
    This result is unexpected. Maybe it's caused by both sides: New RUF needed and old RUF stabilized.
    '''
    TIME_INTERVAL_MONTH = 1
    crate_versionid_relation = get_crate_versionid_relation_DB()
    usage_count = get_ruf_usage_count_DB()
    version_created_time = get_version_created_time_DB()
    crate_created_time = get_crate_created_time_DB()
    count_intervals = dict() # <interval, [count, ruf_count]>
    for crate_id in crate_versionid_relation:
        versions = crate_versionid_relation[crate_id]
        version_count = len(versions)
        for version_id in versions:
            time_interval_month = timedelta_in_months(version_created_time[version_id], crate_created_time[crate_id])
            ruf_usage_count = 0
            if version_id in usage_count:
                ruf_usage_count = usage_count[version_id]
            intervals = int(time_interval_month / TIME_INTERVAL_MONTH)
            if intervals not in count_intervals:
                count_intervals[intervals] = list()
                count_intervals[intervals].append(0)
                count_intervals[intervals].append(0)
            count_intervals[intervals][0] += 1/version_count
            count_intervals[intervals][1] += ruf_usage_count/version_count
    print(count_intervals)
    
    for interval in sorted(count_intervals.keys()):
        count = count_intervals[interval][0]
        ruf_count = count_intervals[interval][1]
        average = ruf_count / count
        print(f"Interval {interval*TIME_INTERVAL_MONTH} months: {count} crates, {ruf_count} ruf, {average} average")



def ruf_usage_inter_crate_evolution():
    '''
    RUF Usage Evolution: Looked at crates created in different time and one-year duration evolution.
    Are new packages more likely to use ruf?
    No. It shows decline. 
    '''
    TIME_INTERVAL_MONTH = 3
    VERSION_EVOLUTION_MONTH = 12
    earlist_date = get_earliest_crate_crated_time_DB()
    crate_versionid_relation = get_crate_versionid_relation_DB()
    usage_count = get_ruf_usage_count_DB()
    version_created_time = get_version_created_time_DB()
    crate_created_time = get_crate_created_time_DB()
    count_intervals = dict() # <interval, [count, ruf_count]>
    for crate_id in crate_versionid_relation:
        created_time = crate_created_time[crate_id]
        crate_interval = timedelta_in_months(created_time, earlist_date)
        versions = crate_versionid_relation[crate_id]
        intervals = int(crate_interval / TIME_INTERVAL_MONTH)
        version_count = 0
        ruf_count = 0
        for version_id in versions:
            time_interval = timedelta_in_months(version_created_time[version_id], created_time)
            if time_interval > VERSION_EVOLUTION_MONTH: # Only consider versions within 1 year
                continue
            ruf_usage_count = 0
            if version_id in usage_count:
                ruf_usage_count = usage_count[version_id]
            version_count += 1
            ruf_count += ruf_usage_count
        if intervals not in count_intervals:
            count_intervals[intervals] = list()
            count_intervals[intervals].append(0)
            count_intervals[intervals].append(0)
        count_intervals[intervals][0] += 1
        if version_count != 0:
            count_intervals[intervals][1] += ruf_count/version_count
    print(count_intervals)
    
    for interval in sorted(count_intervals.keys()):
        count = count_intervals[interval][0]
        ruf_count = count_intervals[interval][1]
        average = ruf_count / count
        print(f"Interval {interval*3} months: {count} crates, {ruf_count} ruf, {average} average")


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


ruf_usage_lifetime()