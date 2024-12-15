import psycopg2 as pg
from transitions import Machine
import statistics
import numpy as np
import matplotlib.pyplot as plt

# active accepted removed imcomlete unknown
# blue   green    red     yellow    grey
# 0      1         2        3         4
# start/unknown -> active/incomplete -> accepted/removed/unknown

class Status(object):
    pass

states = [
    {'name': 'start'},
    {'name': 'active'},
    {'name': 'incomplete'},
    {'name': 'accepted'},
    {'name': 'removed'},
    {'name': 'unknown'},
    {'name': 'error'},
]

transitions = [
    {'trigger': 'meet_active', 'source': ['start', 'active', 'incomplete', 'unknown'], 'dest': 'active'},
    {'trigger': 'meet_active', 'source': ['accepted', 'removed', 'error'], 'dest': 'error'},

    {'trigger': 'meet_accepted', 'source': ['start', 'unknown', 'active', 'accepted', 'incomplete'], 'dest': 'accepted'},
    {'trigger': 'meet_accepted', 'source': ['removed', 'error'], 'dest': 'error'},
    
    {'trigger': 'meet_removed', 'source': ['start', 'active', 'incomplete', 'removed'], 'dest': 'removed'},
    {'trigger': 'meet_removed', 'source': ['accepted', 'unknown', 'error'], 'dest': 'error'},
    
    {'trigger': 'meet_incomplete', 'source': ['start', 'active', 'incomplete', 'unknown'], 'dest': 'incomplete'},
    {'trigger': 'meet_incomplete', 'source': ['accepted', 'removed', 'error'], 'dest': 'error'},

    {'trigger': 'meet_unknown', 'source': ['start', 'unknown'], 'dest': 'unknown'},
    {'trigger': 'meet_unknown', 'source': ['accepted', 'removed', 'active', 'incomplete', 'error'], 'dest': 'error'},
]

# <dest, [source]>
abnormal_transistions = {
    'active': ['accepted', 'removed'],
    # Currently we do not think None->accepted is abnormal. This type of transition do not affect users.
    'accepted': ['removed'], 
    'removed': ['accepted', None],
    'incomplete': ['accepted', 'removed'],
    None: ['accepted', 'removed', 'active', 'incomplete'],
}

def analysis_one(slist: list) -> bool:
    status = Status()
    Machine(model=status, states=states, transitions=transitions, initial='start')
    for s in slist:
        if status.is_error():
            return False

        if s == 'active': # active
            status.meet_active()
        elif s == 'accepted': # accepted
            status.meet_accepted()
        elif s == 'removed': # removed
            status.meet_removed()
        elif s == 'incomplete': # incomplete
            status.meet_incomplete()
        else: # unknown
            status.meet_unknown()
    return True




def fetch_lifetime() -> list:
    cur = conn.cursor()
    cur.execute("SELECT * FROM feature_timeline")
    return cur.fetchall()


def construct_empty_status_list():
    return {  # <status, [time]>
        'accepted': [],
        'active': [],
        'removed': [],
        'incomplete': [],
        None: [],
    }

def evolution_analysis(lifetime: list):
    '''
    RUF Evolution Analysis: How long to get stabilized/removed.
    '''
    accumulative_distribution = {}  # <status, accumulative_time>
    status_distribution = construct_empty_status_list()  # <status, [time]>
    transition_distribution = {  # <transition, count>
        'accepted': construct_empty_status_list(),
        'active': construct_empty_status_list(),
        'removed': construct_empty_status_list(),
        'incomplete': construct_empty_status_list(),
        None: construct_empty_status_list(),
    }

    for l in lifetime:
        name = l[0]
        slist = l[1:]
        for s in slist:
            accumulative_distribution[s] = accumulative_distribution.get(s, 0) + 1
        last_status = 0
        for idx in range(1, len(slist)):
            cur_status = slist[idx]
            pre_status = slist[idx-1]
            if cur_status != pre_status:
                status_duration = (idx - last_status)*42
                transition_distribution[pre_status][cur_status].append(status_duration)
                status_distribution[pre_status].append(status_duration)
                last_status = idx
        # Last status
        status_duration = (idx - last_status)*42
        status_distribution[slist[-1]].append(status_duration)
        transition_distribution[cur_status][cur_status].append(status_duration)
        
    print("status_distribution:", status_distribution)
    print("transition_distribution:", transition_distribution)
    print("accumulative_distribution:", accumulative_distribution)
    for key, value in accumulative_distribution.items():
        accumulative_distribution[key] = value/len(lifetime)
    print("Avg accumulative_distribution days per ruf:", accumulative_distribution)
    # Plotting status_distribution together using box plots
    fig, ax = plt.subplots(figsize=(12, 5))  # Adjusted figure size

    data = [v for k, v in status_distribution.items() if len(v) > 0 and k not in [None, 'incomplete', 'removed']]
    labels = [k for k, v in status_distribution.items() if len(v) > 0 and k not in [None, 'incomplete', 'removed']]
    boxprops = dict(color='black', facecolor='grey')
    whiskerprops = dict(color='blue')
    capprops = dict(color='blue')
    medianprops = dict(color='white')
    flierprops = dict(markerfacecolor='green', marker='o', markersize=5, linestyle='none')
    ax.boxplot(data, vert=False, patch_artist=True, labels=labels, widths=0.5,
            boxprops=boxprops,medianprops=medianprops)
    ax.set_xlabel('Duration (days)', fontsize=16)
    ax.set_ylabel('Status', fontsize=16)
    ax.tick_params(axis='both', which='major', labelsize=16)

    plt.savefig('status_distributions.pdf', format='pdf')
    plt.show()

    # Avg and Sum
    transition_count = 0
    transition_sum = 0
    for k1, kv in transition_distribution.items():
        for k2, v in kv.items():
            if len(v) == 0:
                continue
            if k1 not in [None, 'accepted']:
                transition_count += len(v)
                transition_sum += sum(v)
            print("transition_distribution {0}->{1}: count {2}, avg {3} days".format(k1, k2, len(v), sum(v)/len(v)))
    print("transition_distribution: total count {0}, avg {1} days".format(transition_count, transition_sum/transition_count))




def abnomral_transition_analysis(lifetime: list):
    '''
    Abnormal Transition Analysis: How many abnormal transitions in the lifetime.
    '''
    error = []
    name = ""
    total_abnormal = 0
    distinct_abnormal = 0
    abnormal_distribution = {}
    for l in lifetime:
        ruf = l[0]
        is_abnormal = False
        already_exists = False
        slist = l[1:]
        for idx in range(1, len(slist)):
            cur_status = slist[idx]
            pre_status = slist[idx-1]
            if pre_status is not None and already_exists == False:
                already_exists = True
            if pre_status is None and cur_status is not None and already_exists == True:
                already_exists = None # reactive
                print("{0} has reactive transition in version 1.{1}.0 with transition {2}->{3}".format(ruf, idx, pre_status, cur_status))
            if pre_status in abnormal_transistions[cur_status]:
                print("{0} has abnormal transition in version 1.{1}.0 with transition {2}->{3}".format(ruf, idx, pre_status, cur_status))
                transition = "{0}->{1}".format(pre_status, cur_status)
                if transition in abnormal_distribution:
                    abnormal_distribution[transition] += 1
                else:
                    abnormal_distribution[transition] = 1
                total_abnormal += 1
                is_abnormal = True
        if is_abnormal or already_exists is None:
            distinct_abnormal += 1
            error.append(ruf)
        if already_exists is None:
            total_abnormal += 1
            reactive_transition = "reactive"
            abnormal_distribution[reactive_transition] = abnormal_distribution.get(reactive_transition, 0) + 1
    print("total abnormal: {0}, distinct abnormal: {1}".format(total_abnormal, distinct_abnormal))
    print("Abnormal Distibution:"abnormal_distribution)


conn = pg.connect(database='crates', user='postgres', password='postgres')
lifetime = fetch_lifetime()
abnomral_transition_analysis(lifetime)
# evolution_analysis(lifetime)




cur = conn.cursor()
cur.execute("CREATE TABLE IF NOT EXISTS public.feature_abnormal (name varchar)")
for e in error:
    cur.execute("INSERT INTO feature_abnormal VALUES ('%s')" % (e))

conn.commit()