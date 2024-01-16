import re
import requests
from datetime import datetime

def get_version_date() -> dict:
    versions_date = dict()
    r = requests.get('https://raw.githubusercontent.com/rust-lang/rust/master/RELEASES.md')
    text = r.text
    versions = re.findall("Version 1\.[0-9]+\.0 \([0-9]+-[0-9]+-[0-9]+\)", text)
    
    # `version` example: "Version 0.1  (2012-01-20)"
    for version in reversed(versions):
        version_list = version.split(' ')
        version_num = version_list[1]
        date_string = version_list[2].strip('(').strip(')')
        date = datetime.strptime(date_string, '%Y-%m-%d')
        versions_date[version_num] = date
    return versions_date


VERSION_DATE = get_version_date()



def version_to_date(minor_version: int):
    '''
    Convert version to date
    '''
    version = '1.' + str(minor_version) + '.0'
    if version in VERSION_DATE:
        return VERSION_DATE[version]
    else:
        return None


def timedelta_in_months(end, start):                                # defining the function
    return 12 * (end.year - start.year) + (end.month - start.month) # returning the calculation