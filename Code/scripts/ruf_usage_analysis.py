from datetime import timedelta
import json

import matplotlib
import seaborn as sns
import matplotlib.pyplot as plt
import pandas as pd
import numpy as np

from utils import *
from utils_DB import *



def ruf_usage_lifetime(dump_file):
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
    # ruf_usage_popular_crates()
    # ruf_usage_intra_crate_evolution()
    # ruf_usage_inter_crate_evolution()

    # 1. RUF Change <-> Usage Change
    ruf_change_analysis(dump_file)
    






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
                



def ruf_change_analysis(dump_file):
    '''
    RUF Change -> Usage Change
    How long does it take for crates to react to RUF change?
    For example, when RUF is removed, how long does it take for crates to remove RUF usage after the status change?
    The RUF usage delay compared to RUF status change can represents how actively developers maintain the RUF usage.

    ```
                                                       Inspect Window                              Inspect Window  
                                                    ┌───────────────────┐                          ┌────────────┐
                                                Tran Point(i)        Tran Point(i+1)    Tran Point(i)        Tran Point(i+1)
                                                    |                   |                   |                   |
    RUF Lifetime (By Compiler Version)          U U U | R R R ..... R R R | U U U       U U U | R R R ..... R R R | U U U
    RUF Usage (By Crate Version Release Time)   ...y y y y ... y n ... n n n ...        ...n n n n y y ... y y y y ...
                                                   |│          | │                                 |            │
                                                  v_j         v_k│                                v_j           │
                                                    └────────────┘                                 └────────────┘
                                                  Usage Change Delay                              Usage Change Delay
    ```
    Transition Point: If the RUF status changes in next compiler version, the current version is a RUF status transition point.
                      If the RUF status is in removed status in the first version, we add a transition point at the beginning.
    Inspect Window: The inspect window reprensent the time window, during which the crate changes the RUF usage in response to the first RUF status transition point. 
                    In the inspect window, we analyze when the crate removes its RUF usage, and the delay compared to the first RUF status transition point or the first time using the RUF.
                    The shorter the delay, the more actively the crate maintains its RUF usage.
                    Inspect window includes all versions of a package that are released between two adjacent transition points, including the latest version before the first transition point.
                    The inspect window starts from the first version using RUF, and ends at the release time of the second transition point.
                    Max(TP(i), v_j) -> TP(i+1)
    Usage Change Delay: Min(TP(i+1), v_{k+1}) - Max(TP(i), v_j)
    RUF Status:                     Stable  , Unstable (+Incomplete), Removed (+Unknown)
    Ideal corresponding RUF Usage:  No      , Yes (or No)           , No
    
    Algorithm:
    1. We first find RUF transition point by compiler version.
    2. Then for each crate using the RUF, we try to find .
    3. Analysis
        Target Issues: Divide by status between transition points, each represents different problems.
                1. Stable (Accepted). Forced nighlty compiler and unnecessary usage.
                2. Removed (Removed + Unknown). Compilation failure.
                3. Unstable (Active + Incomplete). Forced nighlty compiler and unstable functionatility.
        Pattern: Divide by usage change.
                1. Stable (Accepted). Usage Yes -> No. Usage Repair.
                2. Removed (Removed + Unknown). Usage Yes -> No. Usage Revoke.
                3. Unstable (Active + Incomplete). Usage Yes -> No. Usage Remove.
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
    usage_tran_time = timedelta(days=0)
    usage_inspect_window_time = timedelta(days=0)
    usage_repair_time = timedelta(days=0)
    usage_revoke_time = timedelta(days=0)
    usage_remove_time = timedelta(days=0)
    usage_tran_time_distribution = dict()
    usage_repair_time_distribution = dict()
    usage_revoke_time_distribution = dict()
    usage_remove_time_distribution = dict()
    usage_tran_repair_distribution = dict()
    usage_tran_revoke_distribution = dict()
    usage_tran_remove_distribution = dict()
    records = list() # Store metadata of all valid inspect windows. [<tran_type, tran_time, inspect_window_time, durantion_yes, last_usage>]
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
        transition_points.append((len(lifetime)-1, 'End'))
        if len(transition_points) <= 1:
            continue
        print(f"\nRUF {ruf} transition points: {transition_points}")
        crates = get_crates_by_used_ruf_DB(ruf)
        for crate_id in crates:
            # 2. Get insepct window
            inspect_window = list() # [[<version>, <usage>]] index [i][j] means the jth version of ith window.
            versions_date = get_crate_versions_created_time_asc_DB(crate_id)
            is_all_not_used = True
            for i in range(len(transition_points)-1):
                # Find Usage Inspect Window
                inspect_window.append(list())
                start_date = version_to_date(transition_points[i][0])
                end_date = version_to_date(transition_points[i+1][0])
                start_version_idx = len(versions_date)
                end_version_idx = len(versions_date)
                for idx in range(len(versions_date)):
                    (version, date) = versions_date[idx]
                    if date > start_date:
                        start_version_idx = max(idx-1 , 0)
                        break
                for idx in range(len(versions_date)):
                    (version, date) = versions_date[idx]
                    if date > end_date:
                        end_version_idx = idx
                        break
                # Record Inspect Window Usage
                for idx in range(start_version_idx, end_version_idx):
                    (version, date) = versions_date[idx]
                    rufs = get_used_ruf_by_versionid_DB(version)
                    ruf_usage = 'No'
                    if ruf in rufs:
                        ruf_usage = 'Yes'
                        is_all_not_used = False
                    inspect_window[i].append((version, ruf_usage))
                # if crate_id == 1779:
                #     print(f'Transition point: {start_date} to {end_date}')
                #     for idx in range(len(versions_date)):
                #         (version, date) = versions_date[idx]
                #         print(date, end=' ')
                #     print()
                #     start_version_date = versions_date[start_version_idx][1]
                #     end_version_date = versions_date[end_version_idx-1][1]
                #     print(f"Start version date: {start_version_date}, end version date: {end_version_date}")
            # 3. Analyze `transition_points` and `inspect_window
            print(f"Analyze ruf {ruf} in crate {crate_id}")
            for idx in range(len(transition_points) - 1):
                start_date = version_to_date(transition_points[idx][0])
                end_date = version_to_date(transition_points[idx+1][0])
                # print(f"Transition point: {start_date} to {end_date}")
                tran_duration = end_date - start_date
                tran_type = transition_points[idx][1]
                first_stable_time = None
                last_stable_begin_time = None
                duration_yes = timedelta(days=0)
                # if tran_type == 'Removed->Unstable':
                #     continue
                if len(inspect_window[idx]) == 0:
                    continue
                # Analyse within inspect window
                #TODO: Algorithm is wrong. Find way.
                print(f"Transition point: Crate {crate_id} from version {inspect_window[idx][0]} to {inspect_window[idx][-1]}, ruf {tran_type}")
                for (version_id, usage) in inspect_window[idx]:
                    version_date = version_created_time[version_id]
                    if version_date > end_date:
                        print('Error', crate_id, version_date)
                    if usage == 'Yes' and not first_stable_time:
                        first_stable_time = max(start_date, version_date)
                    if usage == 'Yes' and not last_stable_begin_time:
                        last_stable_begin_time = max(start_date, version_date)
                    if last_stable_begin_time and usage == 'No':
                        duration_yes += version_date - last_stable_begin_time
                        last_stable_begin_time = None
                    print(usage, end=' ')
                print()
                if last_stable_begin_time:
                    duration_yes += end_date - last_stable_begin_time
                # No usage throughout between transition points, skip.
                if duration_yes == timedelta(days=0):
                    print(f'No usage throughout between transition points, skip.')
                    continue
                usage_count += 1
                last_usage = inspect_window[idx][-1][1]
                usage_tran_time += end_date - start_date
                usage_inspect_window_time += end_date - first_stable_time
                records.append([tran_type, tran_duration.days, (end_date - first_stable_time).days, duration_yes.days, last_usage])
                usage_tran_time_distribution[tran_duration.days] = usage_tran_time_distribution.get(tran_duration.days, 0) + 1
                if '->Stable' in tran_type and duration_yes > timedelta(days=0):
                    print(f"Duration Stable Yes: {duration_yes}, final usage {last_usage}")
                    usage_repair_count += 1
                    usage_repair_time += duration_yes
                    usage_repair_time_distribution[duration_yes.days] = usage_repair_time_distribution.get(duration_yes.days, 0) + 1
                    usage_tran_repair_distribution[tran_duration.days] = usage_tran_repair_distribution.get(tran_duration.days, [])
                    usage_tran_repair_distribution[tran_duration.days].append(duration_yes.days)
                    if last_usage == 'Yes':
                        usage_not_repair_count += 1
                if '->Removed' in tran_type and duration_yes > timedelta(days=0):
                    print(f"Duration Removed Yes: {duration_yes}, final usage {last_usage}")
                    usage_revoke_count += 1
                    usage_revoke_time += duration_yes
                    usage_revoke_time_distribution[duration_yes.days] = usage_revoke_time_distribution.get(duration_yes.days, 0) + 1
                    usage_tran_revoke_distribution[tran_duration.days] = usage_tran_revoke_distribution.get(tran_duration.days, [])
                    usage_tran_revoke_distribution[tran_duration.days].append(duration_yes.days)
                    if last_usage == 'Yes':
                        usage_not_revoke_count += 1
                if '->Unstable' in tran_type and duration_yes > timedelta(days=0):
                    print(f"Duration Unstable Yes: {duration_yes}, final usage {last_usage}")
                    usage_remove_count += 1
                    usage_remove_time += duration_yes
                    usage_remove_time_distribution[duration_yes.days] = usage_remove_time_distribution.get(duration_yes.days, 0) + 1
                    usage_tran_remove_distribution[tran_duration.days] = usage_tran_remove_distribution.get(tran_duration.days, [])
                    usage_tran_remove_distribution[tran_duration.days].append(duration_yes.days)
                    if last_usage == 'Yes':
                        usage_not_remove_count += 1
    print(f"RUF usage count: {usage_count}")
    print(f"RUF usage repair count: {usage_repair_count}")
    print(f"RUF usage revoke count: {usage_revoke_count}")
    print(f"RUF usage remove count: {usage_remove_count}")
    print(f"RUF usage not repair count: {usage_not_repair_count}")
    print(f"RUF usage not revoke count: {usage_not_revoke_count}")
    print(f"RUF usage not remove count: {usage_not_remove_count}")
    print(f"RUF usage tran time: {usage_tran_time}")
    print(f"RUF usage inspect window time: {usage_inspect_window_time}")
    print(f"RUF usage repair time: {usage_repair_time}")
    print(f"RUF usage revoke time: {usage_revoke_time}")
    print(f"RUF usage remove time: {usage_remove_time}")
    print(f"RUF usage tran time distribution: {usage_tran_time_distribution}")
    print(f"RUF usage repair time distribution: {usage_repair_time_distribution}")
    print(f"RUF usage revoke time distribution: {usage_revoke_time_distribution}")
    print(f"RUF usage remove time distribution: {usage_remove_time_distribution}")
    summary = dict()
    summary['usage_count'] = usage_count
    summary['usage_repair_count'] = usage_repair_count
    summary['usage_revoke_count'] = usage_revoke_count
    summary['usage_remove_count'] = usage_remove_count
    summary['usage_not_repair_count'] = usage_not_repair_count
    summary['usage_not_revoke_count'] = usage_not_revoke_count
    summary['usage_not_remove_count'] = usage_not_remove_count
    summary['usage_tran_time'] = usage_tran_time.days
    summary['usage_inspect_window_time'] = usage_inspect_window_time.days
    summary['usage_repair_time'] = usage_repair_time.days
    summary['usage_revoke_time'] = usage_revoke_time.days
    summary['usage_remove_time'] = usage_remove_time.days
    summary['usage_tran_time_distribution'] = usage_tran_time_distribution
    summary['usage_repair_time_distribution'] = usage_repair_time_distribution
    summary['usage_revoke_time_distribution'] = usage_revoke_time_distribution
    summary['usage_remove_time_distribution'] = usage_remove_time_distribution
    summary['usage_tran_repair_distribution'] = usage_tran_repair_distribution
    summary['usage_tran_revoke_distribution'] = usage_tran_revoke_distribution
    summary['usage_tran_remove_distribution'] = usage_tran_remove_distribution
    summary['records'] = records

    with open(dump_file, 'w') as f:
        json.dump(summary, f, indent=4)
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



def process_results(dump_file):
    '''
    Process results dumped by `ruf_change_analysis()`.
    RUF Usage Crate Count: `SELECT COUNT(DISTINCT crate_id) FROM version_feature_ori INNER JOIN versions ON version_feature_ori.id = versions.id WHERE feature IS NOT NULL`
    '''
    with open(dump_file, 'r') as f:
        summary = json.load(f)
    usage_count = summary['usage_count']
    usage_repair_count = summary['usage_repair_count']
    usage_revoke_count = summary['usage_revoke_count']
    usage_remove_count = summary['usage_remove_count']
    usage_not_repair_count = summary['usage_not_repair_count']
    usage_not_revoke_count = summary['usage_not_revoke_count']
    usage_not_remove_count = summary['usage_not_remove_count']
    usage_tran_time = summary['usage_tran_time']
    usage_inspect_window_time = summary['usage_inspect_window_time']
    usage_repair_time = summary['usage_repair_time']
    usage_revoke_time = summary['usage_revoke_time']
    usage_remove_time = summary['usage_remove_time']
    usage_tran_time_distribution = summary['usage_tran_time_distribution']
    usage_repair_time_distribution = summary['usage_repair_time_distribution']
    usage_revoke_time_distribution = summary['usage_revoke_time_distribution']
    usage_remove_time_distribution = summary['usage_remove_time_distribution']
    usage_tran_repair_distribution = summary['usage_tran_repair_distribution']
    usage_tran_revoke_distribution = summary['usage_tran_revoke_distribution']
    usage_tran_remove_distribution = summary['usage_tran_remove_distribution']
    records = summary['records']
    print(f"RUF usage count: {usage_count}")
    print(f"RUF usage repair count: {usage_repair_count}")
    print(f"RUF usage revoke count: {usage_revoke_count}")
    print(f"RUF usage remove count: {usage_remove_count}")
    print(f"RUF usage not repair count: {usage_not_repair_count}")
    print(f"RUF usage not revoke count: {usage_not_revoke_count}")
    print(f"RUF usage not remove count: {usage_not_remove_count}")
    print(f"RUF usage tran time: {usage_tran_time} days")
    print(f"RUF usage inspect window time: {usage_inspect_window_time} days")
    print(f"RUF usage repair time: {usage_repair_time} days")
    print(f"RUF usage revoke time: {usage_revoke_time} days")
    print(f"RUF usage remove time: {usage_remove_time} days")
    print(f"RUF usage tran average time: {usage_tran_time / usage_count} days")
    print(f"RUF usage inspect window average time: {usage_inspect_window_time / usage_count} days")
    print(f"RUF usage repair average time: {usage_repair_time / usage_repair_count} days")
    print(f"RUF usage revoke average time: {usage_revoke_time / usage_revoke_count} days")
    print(f"RUF usage remove average time: {usage_remove_time / usage_remove_count} days")
    print(f"RUF usage tran time distribution: {usage_tran_time_distribution}")
    print(f"RUF usage repair time distribution: {usage_repair_time_distribution}")
    print(f"RUF usage revoke time distribution: {usage_revoke_time_distribution}")
    print(f"RUF usage remove time distribution: {usage_remove_time_distribution}")
    print(f"RUF usage tran repair distribution: {usage_tran_repair_distribution}")
    print(f"RUF usage tran revoke distribution: {usage_tran_revoke_distribution}")
    print(f"RUF usage tran remove distribution: {usage_tran_remove_distribution}")
    # Accumulative distribution: 
    # times = [42, 84, 126, 180, 270, 360, 450, 540, 720, 900, 1080, 1440, 1800, 2160, 2520, 2880]
    times = [90, 180, 270, 360, 450, 540, 720, 900, 1080, 1440, 1800, 2160, 2520, 2880]
    accu_tran_time_distribution = dict()
    accu_repair_time_distribution = dict()
    accu_revoke_time_distribution = dict()
    accu_remove_time_distribution = dict()
    for usage_tran_time in usage_tran_time_distribution:
        count = usage_tran_time_distribution[usage_tran_time]
        usage_tran_time = int(usage_tran_time)
        for time in times:
            if usage_tran_time <= time:
                accu_tran_time_distribution[time] = accu_tran_time_distribution.get(time, 0) + count
                break
    for usage_repair_time in usage_repair_time_distribution:
        count = usage_repair_time_distribution[usage_repair_time]
        usage_repair_time = int(usage_repair_time)
        for time in times:
            if usage_repair_time <= time:
                accu_repair_time_distribution[time] = accu_repair_time_distribution.get(time, 0) + count
                break
    for usage_revoke_time in usage_revoke_time_distribution:
        count = usage_revoke_time_distribution[usage_revoke_time]
        usage_revoke_time = int(usage_revoke_time)
        for time in times:
            if usage_revoke_time <= time:
                accu_revoke_time_distribution[time] = accu_revoke_time_distribution.get(time, 0) + count
                break
    for usage_remove_time in usage_remove_time_distribution:
        count = usage_remove_time_distribution[usage_remove_time]
        usage_remove_time = int(usage_remove_time)
        for time in times:
            if usage_remove_time <= time:
                accu_remove_time_distribution[time] = accu_remove_time_distribution.get(time, 0) + count
                break
    print("Days, Tran, Repair, Revoke, Remove")
    for time in times:
        tran = accu_tran_time_distribution.get(time, 0)
        repair = accu_repair_time_distribution.get(time, 0)
        revoke = accu_revoke_time_distribution.get(time, 0)
        remove = accu_remove_time_distribution.get(time, 0)
        print(f"{time}, {tran}, {repair}, {revoke}, {remove}")
    # Distribution of transition and repair/revoke/remove time
    summary_distribution = dict()
    for usage_tran_time in usage_tran_time_distribution:
        if usage_tran_time not in usage_tran_repair_distribution:
            repair_average_time = 0
        else:
            repair_average_time = sum(usage_tran_repair_distribution[usage_tran_time]) / len(usage_tran_repair_distribution[usage_tran_time])
        if usage_tran_time not in usage_tran_revoke_distribution:
            revoke_average_time = 0
        else:
            revoke_average_time = sum(usage_tran_revoke_distribution[usage_tran_time]) / len(usage_tran_revoke_distribution[usage_tran_time])
        if usage_tran_time not in usage_tran_remove_distribution:
            remove_average_time = 0
        else:
            remove_average_time = sum(usage_tran_remove_distribution[usage_tran_time]) / len(usage_tran_remove_distribution[usage_tran_time])
        summary_distribution[int(usage_tran_time)] = [repair_average_time, revoke_average_time, remove_average_time]
    
    print("Tran, Repair, Revoke, Remove")
    for usage_tran_time in sorted(summary_distribution):
        (repair_average_time, revoke_average_time, remove_average_time) = summary_distribution[usage_tran_time]
        print(f"{usage_tran_time}, {repair_average_time}, {revoke_average_time}, {remove_average_time}")
    # Distribution of inspect window and repair/revoke/remove time
    print("Inspect Window, Repair, Revoke, Remove")
    processed_records = list()
    for record in records:
        tran_type = record[0]
        tran_time = record[1]
        inspect_window_time = record[2]
        duration_yes = record[3]
        last_usage = record[4]
        # if duration_yes + 1 >= inspect_window_time:
        #     last_usage = 1
        # else:
        #     last_usage = 0
        if last_usage == 'Yes':
            last_usage = 1
        else:
            last_usage = 0
        if last_usage == 'Yes': # Given more time, fix delay can be longer. We only consider fixed cases.
            continue
        type = None
        if '->Stable' in tran_type:
            type = 'Stable'
        if '->Removed' in tran_type:
            type = 'Removed'
        if '->Unstable' in tran_type:
            type = 'Unstable'
        another_record = [type, inspect_window_time, duration_yes, last_usage]
        processed_records.append(another_record)
        # print(f"{inspect_window_time}, {duration_yes}, {type}")
    make_graph(processed_records)


def make_graph(records):

    sns.set_style("whitegrid")

    # pal = dict(male="#6495ED", female="#F08080")
    # g = sns.lmplot(x="age", y="survived", col="sex", hue="sex", data=df,
    #             palette=pal, y_jitter=.02, logistic=True, truncate=False)
    
    column = ['ruf_type',  'inspect_window_time', 'duration_yes', 'last_usage']
    df = pd.DataFrame(data = records, columns=column)
    print(df)
    pal = {'Unstable':"#6495ED", 'Removed':"#F08080", 'Stable':'#80d819'}

    # Figure 1: Window <-> Last Usage
    # plt.figure().clear()
    # g = sns.lmplot(x="inspect_window_time", y="last_usage", col="ruf_type", hue="ruf_type", data=df,
    #             palette=pal, y_jitter=.02, logistic=True , truncate=False)
    # g.set(xlim=(0, 3000), ylim=(-.05, 1.05))
    # matplotlib.pyplot.savefig('figure1.pdf', dpi=400, format='pdf')

    # # Figure 2: Window <-> Duration Yes
    # plt.figure().clear()
    # g = sns.lmplot(
    #     data=df,
    #     x="inspect_window_time", y="duration_yes", hue="ruf_type", palette=pal,
    #     height=5
    # )
    # matplotlib.pyplot.savefig('figure2.pdf', dpi=400, format='pdf')

    # Figure 3: Window <-> Duration Yes (6wks, last usage True)
    # plt.figure().clear()
    # records_window_6wks_records = list()
    # for record in records:
    #     ruf_type = record[0]
    #     inspect_window_time = int(record[1]/42)
    #     duration_yes = record[2]
    #     last_usage = record[3]
    #     if last_usage == 1:
    #         records_window_6wks_records.append([ruf_type, inspect_window_time, duration_yes, last_usage])
    # df = pd.DataFrame(data = records_window_6wks_records, columns=column)
    # g = sns.lmplot(
    #     data=df,
    #     x="inspect_window_time", y="duration_yes", col="ruf_type", hue="ruf_type", palette=pal,
    #     height=5
    # )
    # matplotlib.pyplot.savefig('figure3.pdf', dpi=400, format='pdf')

    # Figure 4: Window <-> Duration Yes (last usage False)
    # plt.figure().clear()
    # records_window_6wks_records = list()
    # for record in records:
    #     ruf_type = record[0]
    #     inspect_window_time = record[1]/365
    #     duration_yes = record[2]/365
    #     last_usage = record[3]
    #     if last_usage == 0:
    #         records_window_6wks_records.append([ruf_type, inspect_window_time, duration_yes, last_usage])
    # df = pd.DataFrame(data = records_window_6wks_records, columns=column)
    # print(df)
    # g = sns.lmplot(
    #     data=df,
    #     x="inspect_window_time", y="duration_yes", col="ruf_type", hue="ruf_type", palette=pal,lowess=True,
    #     scatter_kws={"color": "#BFBFBF", "s": 10}, line_kws={"linewidth": 3},height=6, aspect=0.7
    # )
    # g.set(xlim=(0, 8), ylim=(0, 5))
    # plt.axhline(y=0.2)
    # matplotlib.pyplot.savefig('figure4.pdf', dpi=400, format='pdf')


    # Figure 5: Window <-> Average Duration Yes (6wks, last usage False)
    plt.figure().clear()
    column = ['ruf_type',  'inspect_window_time', 'duration_yes']
    graph_data = list()
    graph_data.extend(process_average_window(records, 'Stable'))
    graph_data.extend(process_average_window(records, 'Unstable'))
    graph_data.extend(process_average_window(records, 'Removed'))
    df = pd.DataFrame(data = graph_data, columns=column)
    g = sns.lmplot(
        data=df,
        x="inspect_window_time", y="duration_yes", col="ruf_type", hue="ruf_type", palette=pal, lowess=True,
        height=5
    )
    matplotlib.pyplot.savefig('figure5.png', dpi=400, format='png')


    # Figure 6: Window <-> Average Duration Yes (6wks, last usage False) log regression
    # plt.figure().clear()
    # column = ['ruf_type',  'inspect_window_time', 'duration_yes', 'last_usage']
    # records_window_6wks_records = list()
    # for record in records:
    #     ruf_type = record[0]
    #     inspect_window_time = int(record[1]/84) + 1
    #     duration_yes = record[2]
    #     last_usage = record[3]
    #     if last_usage == 0:
    #         records_window_6wks_records.append([ruf_type, inspect_window_time, duration_yes, last_usage])
    # df = pd.DataFrame(data = records_window_6wks_records, columns=column)
    # g = sns.lmplot(
    #     data=df,
    #     x="inspect_window_time", y="duration_yes", col="ruf_type", hue="ruf_type", palette=pal, x_estimator=np.mean, lowess=True, 
    #     height=5
    # )
    # matplotlib.pyplot.savefig('figure6.pdf', dpi=400, format='pdf')

    # Box Plot
    # Draw a nested boxplot to show bills by day and time
    plt.figure().clear()
    column = ['ruf_type',  'inspect_window_time', 'duration_yes', 'last_usage']
    average_dots = list()
    for record in records:
        ruf_type = record[0]
        inspect_window_time = int(record[1]/84)
        duration_yes = record[2]
        last_usage = record[3]
        if ruf_type != 'Stable':
            continue
        if last_usage == 0:
            average_dots.append([ruf_type, inspect_window_time, duration_yes, last_usage])
    df = pd.DataFrame(data = average_dots, columns=column)
    sns.boxplot(x="inspect_window_time", y="duration_yes",
                color="#6495ED", flierprops={'markerfacecolor': '#00000000', 'markeredgecolor': '#00000000'},boxprops=dict(edgecolor='#6495ED'),
                data=df)
    matplotlib.pyplot.savefig('Box.png', dpi=400, format='png')
    sns.despine(offset=10, trim=True)


    # Final figure
    plt.figure().clear()
    ruf_types = ['Stable', 'Unstable', 'Removed']
    figure, axes = plt.subplots(nrows=2, ncols=3, figsize=(12, 10), height_ratios=[1, 1.8], sharey='row', sharex='col')
    for idx in range(len(ruf_types)):
        ruf_type = ruf_types[idx]
        color = pal[ruf_type]
        # Top figure: Window <-> Last Usage
        column = ['ruf_type',  'inspect_window_time', 'duration_yes', 'last_usage']
        df = pd.DataFrame(data = records, columns=column)
        df = df[df['ruf_type'] == ruf_type]
        g = sns.regplot(x="inspect_window_time", y="last_usage", data=df,
                scatter_kws={"color": "#BFBFBF", "s": 3}, y_jitter=.03, logistic=True, ax= axes[0, idx], color=color)
        # Layer 1: Dots
        dots = process_wk_window(records, ruf_type)
        column = ['ruf_type',  'inspect_window_time', 'duration_yes']
        df = pd.DataFrame(data = dots, columns=column)
        g = sns.scatterplot(
            data=df,
            x="inspect_window_time", y="duration_yes", 
            color='#BFBFBF', size = 10, ax = axes[1, idx], legend= False
        )
        # Layer 2: Lowess Regression of Average Duration Yes
        graph_data = process_average_window(records, ruf_type)
        df = pd.DataFrame(data = graph_data, columns=column)
        g = sns.regplot(
            data=df,
            x="inspect_window_time", y="duration_yes", lowess=True, marker='D', color=color,
            scatter_kws={"s": 35}, line_kws={"linewidth": 3}, ax = axes[1, idx]
        )
        g.set(xlim=(0, 2800), ylim=(0, 1000))
    for ax, title in zip(axes[0], ruf_types):
        ax.set_title(title + ' RUF')
        ax.set(xlabel='', ylabel='')
    for ax, title in zip(axes[1], ruf_types):
        ax.set(xlabel='Fix Window / days', ylabel='')
    axes[0, 0].set(ylabel='Final Usage')
    axes[1, 0].set(ylabel='Fix Time / days')
    # plt.tight_layout()
    plt.subplots_adjust(wspace=0.07, hspace=0.07)
    matplotlib.pyplot.savefig('ruf_repair_yes.png', dpi=400, format='png')
    matplotlib.pyplot.savefig('ruf_repair_yes.pdf', dpi=400, format='pdf')




def process_average_window(records, ruf_type):
    ''' Internal Use'''
    records_window_6wks_records = [(0,0)] * int(3000/42)
    for record in records:
        record_ruf_type = record[0]
        if record_ruf_type != ruf_type:
            continue
        inspect_window_time = int(record[1]/42)
        duration_yes = record[2]
        last_usage = record[3]
        count = records_window_6wks_records[inspect_window_time][0]
        total = records_window_6wks_records[inspect_window_time][1]
        if last_usage == 0:
            records_window_6wks_records[inspect_window_time] = (count+1, total+duration_yes)
    average_result = list()
    last_average = 0
    for idx in range(len(records_window_6wks_records)):
        (count, total) = records_window_6wks_records[idx]
        if count != 0:
            average = int(total / count)
            average_result.append([ruf_type, idx*42, average])
        else :
            average = last_usage
        last_usage = average
    return average_result


def process_wk_window(records, ruf_type):
    ''' Internal Use'''
    records_window_6wks_records = list()
    for record in records:
        record_ruf_type = record[0]
        if record_ruf_type != ruf_type:
            continue
        inspect_window_time = int(record[1]/42)*42
        duration_yes = record[2]
        last_usage = record[3]
        if last_usage == 0:
            records_window_6wks_records.append([ruf_type, inspect_window_time, duration_yes])
    return records_window_6wks_records



import sys


if len(sys.argv) < 2:
    print('Usage: python3 ruf_usage_analysis.py [ruf_usage_lifetime | results]')
    exit()
if sys.argv[1] == 'ruf_usage_lifetime':
    ruf_usage_lifetime('ruf_change_analysis.json')
elif sys.argv[1] == 'results':
    process_results('ruf_change_analysis.json')
else:
    print('Invalid command')
    print('Usage: python3 ruf_usage_analysis.py [ruf_usage_lifetime | results]')

