from datetime import timedelta

from utils import *
from utils_DB import *



def ruf_usage_lifetime():
    '''
    Main function.
    We mainly analyze how RUF is used by developers and how they maintain the RUF.
    0. (Overview) How many packages use RUF? Are they using more through time? What about packages with large impacts (downloads).
        We should not only focus on old crates releasing new versions, but also new crates seperately. This will reveal the true new usage.
        Differentiate new crates and new versions.
    1. (RUF Change <-> Usage Change) RUF can be removed. How do they react to RUF removal? Do they update their code?
        RUF can be stabilized. Removing RUF usage can prevent nightly compiler selection.
        1.1 For extremly abnormal sequence. (Stable -> Unstable -> Stable, unstable -> unknown -> unstable, ...). How do packages react.
        1.2 Will developers try to actively remove RUF usage actively rather than after RUF removal?
            If so, how do they do it? Do they remove RUF usage only when RUF are stabilized or removed?
    2. RUF fix. (Pending) Will developers backport their fix to old versions? Fix with minor/major version update may not apply to old versions.
        How many versions are still suffer? This is the critical point in semver-based ecosytem. 
    '''
    # 0. Overview
    ruf_usage_popular_crates()
    ruf_usage_intra_crate_evolution()
    ruf_usage_inter_crate_evolution()

    # 1. RUF Change <-> Usage Change
    ruf_change_analysis()
    






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



def ruf_mismatch_overview():
    '''
    RUF Status <-> Ideal Usage. How much accumulation time do crates use RUF in the ideal way?
    Pending. The `ruf_change_analysis()` seems to be enough for the analysis.
    '''
    RUF_STATUS = {
        'accepted': 'Stable',
        'active': 'Unstable',
        'incomplete': 'Unstable',
        'removed': 'Removed',
    }
    ruf_lifetime = get_ruf_lifetime_DB()
    version_created_time = get_version_created_time_DB()
    usage_count = 0
    for ruf in ruf_lifetime:
        lifetime = ruf_lifetime[ruf]
        crates = get_crates_by_used_ruf_DB(ruf)
        for crate_id in crates:
            versions_date = get_crate_versions_created_time_asc_DB(crate_id)
            for (version, date) in versions_date:
                rufs = get_used_ruf_by_versionid_DB(version)
                ruf_usage = 'No'
                if ruf in rufs:
                    ruf_usage = 'Yes'
                



def ruf_change_analysis():
    '''
    RUF Change -> Usage Change
    Transition reaction delay. How long does it take for crates to react to RUF change?

    ```
                                                Tran Point          Tran Point
                                                    |                 |
    RUF Lifetime (By Release Version)           A A A | B B B ... B B B | C C C
    RUF Usage (By Crate Version Release Time)   ...a a a a a a ... b b b x x x x
                                                     └───────────────┘
                                                    Usage Inspect Window
    ```
    RUF Status:                     Stable  , Unstable (+Incomplete), Removed (+Unknown)
    Ideal corresponding RUF Usage:  No      , Yes (or No)           , No
    
    Algorithm:
    1. We first find RUF transition point (A-> B, B-> C) by release version.
    2. Then we try to find RUF usage transition point (a-> b).
    3. Analysis
        Target Issues: Divide by transition forms. Each represents different problems.
                1. Unstable -> Stable. Forced nighlty compiler.
                2. Unstable -> Removed. Compilation failure.
                3. Extremely abnormal sequence. Seperate analysis. We do not analyze them, as the the problem mainly comes from RUF side.
                    3.1 Stable -> Unstable. Functionality breaks. Need to add RUF usage.
                    3.2 Stable -> Removed. Nothing special.
        Pattern: Divide by usage change.
                1. RUF Unstable->Stable. Usage Yes -> No. Usage Repair.
                2. RUF Unstable->Removed. Usage Yes -> No. Usage Revoke.
                3. Unstable-Unstable. Usage Yes -> No. Usage Remove.
    4. 2022-08-11 Result:
        Overview: The packages react poorly to RUF change. They do not actively detect and repair RUF.
    '''
    RUF_STATUS = {
        'accepted': 'Stable',
        'active': 'Unstable',
        'incomplete': 'Unstable',
        'removed': 'Removed',
    }
    ruf_lifetime = get_ruf_lifetime_DB()
    version_created_time = get_version_created_time_DB()
    usage_count = 0
    usage_repair_count = 0
    usage_revoke_count = 0
    usage_remove_count = 0
    usage_not_repair_count = 0
    usage_not_revoke_count = 0
    usage_not_remove_count = 0
    usage_repair_time = timedelta(days=0)
    usage_revoke_time = timedelta(days=0)
    usage_remove_time = timedelta(days=0)
    usage_repair_time_distribution = dict()
    usage_revoke_time_distribution = dict()
    usage_remove_time_distribution = dict()
    for ruf in ruf_lifetime:
        lifetime = ruf_lifetime[ruf]
        # 1. Find RUF transition point
        transition_points = list() # [<version>, <transition_type>]
        begin_status = RUF_STATUS.get(lifetime[0], 'Removed')
        if begin_status != 'Removed':
            transition_points.append((0, 'Removed->'+ begin_status))
        for i in range(0, len(lifetime)-1):
            before =  RUF_STATUS.get(lifetime[i], 'Removed')
            after = RUF_STATUS.get(lifetime[i+1], 'Removed')
            if before != after:
                transition_points.append((i, before + '->' + after))
        transition_points.append((len(lifetime), 'End'))
        if len(transition_points) <= 1:
            continue
        print(f"\nRUF {ruf} transition points: {transition_points}")
        # 2. Analysis
        crates = get_crates_by_used_ruf_DB(ruf)
        for crate_id in crates:
            inspect_window = list() # [[<version>, <usage>]] index [i][j] means the jth version of ith window.
            versions_date = get_crate_versions_created_time_asc_DB(crate_id)
            is_all_not_used = True
            for i in range(len(transition_points)-1):
                # Find Usage Inspect Window
                inspect_window.append(list())
                start_date = version_to_date(transition_points[i][0])
                start_version_idx = 0
                for idx in range(len(versions_date)):
                    (version, date) = versions_date[idx]
                    if date > start_date:
                        start_version_idx = idx
                        break
                end_date = version_to_date(transition_points[i+1][0])
                end_version_idx = len(versions_date) - 1
                for idx in range(len(versions_date)):
                    (version, date) = versions_date[idx]
                    if date > end_date:
                        end_version_idx = max(idx - 1, 0)
                        break
                # Record Inspect Window Usage
                for idx in range(start_version_idx, end_version_idx+1):
                    (version, date) = versions_date[idx]
                    rufs = get_used_ruf_by_versionid_DB(version)
                    ruf_usage = 'No'
                    if ruf in rufs:
                        ruf_usage = 'Yes'
                        is_all_not_used = False
                    inspect_window[i].append((version, ruf_usage))
            # Analyze `transition_points` and `inspect_window
            print(f"Analyze ruf {ruf} in crate {crate_id}")
            for idx in range(len(transition_points) - 1):
                usage_count += 1

                start_date = version_to_date(transition_points[i][0])
                end_date = version_to_date(transition_points[i+1][0])
                tran_type = transition_points[idx][1]
                first_stable_time = None
                duration_stable_yes = timedelta(days=0)
                # if tran_type == 'Removed->Unstable':
                #     continue
                if len(inspect_window[idx]) == 0:
                    continue
                # Analyse within inspect window
                print(f"Transition point: Crate {crate_id} from version {inspect_window[idx][0]} to {inspect_window[idx][-1]}, ruf {tran_type}")
                for (version_id, usage) in inspect_window[idx]:
                    version_date = version_created_time[version_id]
                    if usage == 'Yes' and not first_stable_time:
                        first_stable_time = version_date
                    if first_stable_time and usage == 'No':
                        duration_stable_yes += version_date - first_stable_time
                        first_stable_time = None
                    print(usage, end=' ')
                print()
                last_usage = inspect_window[idx][-1][1]
                if first_stable_time:
                    duration_stable_yes += end_date - first_stable_time
                if tran_type == 'Unstable->Stable' and duration_stable_yes > timedelta(days=0):
                    print(f"Duration Stable Yes: {duration_stable_yes}, final usage {last_usage}")
                    usage_repair_count += 1
                    usage_repair_time += duration_stable_yes
                    usage_repair_time_distribution[duration_stable_yes.days] = usage_repair_time_distribution.get(duration_stable_yes.days, 0) + 1
                    if last_usage == 'No':
                        usage_not_repair_count += 1
                if tran_type == 'Unstable->Removed' and duration_stable_yes > timedelta(days=0):
                    print(f"Duration Removed Yes: {duration_stable_yes}, final usage {last_usage}")
                    usage_revoke_count += 1
                    usage_revoke_time += duration_stable_yes
                    usage_revoke_time_distribution[duration_stable_yes.days] = usage_revoke_time_distribution.get(duration_stable_yes.days, 0) + 1
                    if last_usage == 'No':
                        usage_not_revoke_count += 1
                if tran_type == 'Removed->Unstable' and duration_stable_yes > timedelta(days=0):
                    print(f"Duration Unstable Yes: {duration_stable_yes}, final usage {last_usage}")
                    usage_remove_count += 1
                    usage_remove_time += duration_stable_yes
                    usage_remove_time_distribution[duration_stable_yes.days] = usage_remove_time_distribution.get(duration_stable_yes.days, 0) + 1
                    if last_usage == 'No':
                        usage_not_remove_count += 1
    print(f"RUF usage count: {usage_count}")
    print(f"RUF usage repair count: {usage_repair_count}")
    print(f"RUF usage revoke count: {usage_revoke_count}")
    print(f"RUF usage remove count: {usage_remove_count}")
    print(f"RUF usage not repair count: {usage_not_repair_count}")
    print(f"RUF usage not revoke count: {usage_not_revoke_count}")
    print(f"RUF usage not remove count: {usage_not_remove_count}")
    print(f"RUF usage repair time: {usage_repair_time}")
    print(f"RUF usage revoke time: {usage_revoke_time}")
    print(f"RUF usage remove time: {usage_remove_time}")
    print(f"RUF usage repair time distribution: {usage_repair_time_distribution}")
    print(f"RUF usage revoke time distribution: {usage_revoke_time_distribution}")
    print(f"RUF usage remove time distribution: {usage_remove_time_distribution}")
            # Commonly seen in crates older than first stable compiler. This is OK. We do not analyze them (before first stable Rust).
            # if is_all_not_used:
            #     print(f"!!!!!WARNING!!!!! RUF {ruf} in crate {crate_id} is not used throughout the window.")
            #     print(f"!!!!!!Debug!!!!!! Version uses RUF in: ", end='')
            #     for idx in range(len(versions_date)):
            #         (version, date) = versions_date[idx]
            #         rufs = get_used_ruf_by_versionid_DB(version)
            #         if ruf in rufs:
            #             print(version, end=' ')
            #     print()
                

ruf_usage_lifetime()