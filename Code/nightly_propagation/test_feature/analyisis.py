import psycopg2 as pg
from transitions import Machine

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

    {'trigger': 'meet_unknown', 'source': ['start', 'active', 'incomplete', 'unknown', 'removed'], 'dest': 'unknown'},
    {'trigger': 'meet_unknown', 'source': ['accepted', 'error'], 'dest': 'error'},
]

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


conn = pg.connect(database='crates_08_22', user='postgres', password='postgres')

error = []
lifetime = fetch_lifetime()
for l in lifetime:
    if not analysis_one(l[1:]):
        error.append(l[0])

cur = conn.cursor()
cur.execute("CREATE TABLE IF NOT EXISTS public.feature_abnormal (name varchar)")
for e in error:
    cur.execute("INSERT INTO feature_abnormal VALUES ('%s')" % (e))

conn.commit()