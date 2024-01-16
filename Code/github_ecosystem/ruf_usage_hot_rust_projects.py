from glob import glob
import psycopg2
import os
import sys
import re
import subprocess
import json

target_file_name = 'rust_hot_projects_20220811.md'
output_dir = './projects'
time = '2022-08-11 00:00:00 +0000'
analyze_path = '/home/loancold/Projects/Cargo-Ecosystem-Monitor/Cargo-Ecosystem-Monitor/rust/build/x86_64-unknown-linux-gnu/stage1/bin/rustc' # The path of our ruf usage analyzer (modified rustc)
# case_study_paths = ['./Rust4Linux/linux', './AOSP', './Firefox/mozilla-unified']
case_study_paths = ['./AOSP', './Firefox/git_firefox', './Rust4Linux/linux']

conn = psycopg2.connect(
    host="localhost",
    database="crates",
    user="postgres",
    password="postgres"
)



# regex parse example: | 1 | [deno](https://github.com/denoland/deno) | 84481 | 4552 |
def parse_top_projects(target_file_name: str) -> list:
    url_list = list()
    text = open(target_file_name, 'r').read()
    projects = re.findall("\| \d+ \| \[.+\]\(https://github.com/[\w/\-\.]+\) \| \d+ \| \d+ \|", text)
    if len(projects) != 100:
        print('Some hot projects are missing')
    for idx, project in enumerate(projects):
        re_result = re.findall("\| (\d+) ", project)
        rank = re_result[0]
        stars = re_result[1]
        fork = re_result[2]
        url = re.search('\(https://github.com/[\w/\-\.]+\)', project)[0][1:-1]
        name = re.search('\[.+\]', project)[0][1:-1]
        print(f'rank: {rank}, stars: {stars}, fork: {fork}, url: {url}, name: {name}')
        if int(rank) != idx + 1:
            print('Rank is not correct', rank, idx + 1)
        url_list.append(url)
    return url_list
    

def download_github_projects(url_list: list):
    if not os.path.exists(output_dir):
        os.mkdir(output_dir)
    for url in url_list:
        print(f'Cloning {url}')
        os.system(f"cd {output_dir} && git clone {url}")


def checkout_to_time():
    # Checkout to the latest commit before the time
    for dir in os.listdir(f'./{output_dir}'):
        print(f'Checkout {dir}')
        branch = subprocess.getoutput(f"cd {output_dir}/{dir} && git branch")[2:]
        os.system(f"cd {output_dir}/{dir} && git checkout `git rev-list -n 1 --first-parent --before=\"{time}\" {branch}`")


def analyze_github_projects():
    ruf_usage = dict()
    absolute_output_dir = os.getcwd() + '/' + output_dir
    for dir in os.listdir(absolute_output_dir):
        project_name = dir
        target_dir = f'{absolute_output_dir}/{dir}'
        # print(f'Analyzing {target_dir}')
        # for file_name in glob(target_dir + '/**/*.rs', recursive=True):
        for file_name in glob(target_dir + '/**/lib.rs', recursive=True):
            if 'test' in file_name:
                continue
            results = subprocess.run([analyze_path, file_name, '--edition', '2021', '--ruf-analysis'], stdout=subprocess.PIPE).stdout.decode('utf-8')
            re_ori = re.findall("formatori \(\[(.*?)\], (.*?)\)", results)
            re_processed = re.findall("processed \(\[(.*?)\], (.*?)\)", results)
            if len(re_ori) != 0:
                # print(f'Analyzing {target_dir}')
                for item in re_ori:
                    cond = item[0]
                    ruf = item[1]
                    # if ruf == 'test' or ruf == 'doc_cfg':
                    #     continue
                    if not ruf_usage.get(project_name):
                        ruf_usage[project_name] = set()
                    ruf_usage[project_name].add(ruf)
                # print(file_name)
                # print(re_ori)
    for (key, value) in ruf_usage.items():
        print(f'{key}: {value}')


def get_dependency_tree(toml_path: str) -> dict:
    '''
    Resolve and extract project dependency and enabled features.
    Crates typically use edition 2021. We first modify the Cargo.toml to use edition 2021.
    We get dependency by using `cargo tree -e no-dev` and features by using `cargo tree -e no-dev -f {f}`.
    Noticed Issue: `Cargo Tree` is not stable. The results will slightly changed when you run it multiple times. We compare to ensure the results are the same.
    '''
    with open(toml_path, 'r') as f:
        toml = f.read()
        f.close()
        if 'resolver = ' not in toml:
            with open(toml_path, 'w') as f:
                toml = toml.replace('[workspace]', '[workspace]\nresolver = "2"')
                f.write(toml)
    dependency_trees = dict()
    subprocess.run(['cargo', 'update', '--manifest-path', toml_path], stdout=subprocess.PIPE).stdout.decode('utf-8')
    dependency_results = subprocess.run(['cargo', 'tree', '--manifest-path', toml_path, '-e', 'no-dev', '--all-features', '--target', 'all', '-f','"{p} {f}"'], stdout=subprocess.PIPE).stdout.decode('utf-8')
    # print('---------dependency_results----------')
    # print(dependency_results)
    # print('---------analysis----------')
    dependency_lines = dependency_results.split('\n')
    for line in dependency_lines:
        line = line.replace('"', '')
        version = re.search("[\w-]+ v[0-9]+.[0-9]+.[0-9]+[\S]*", line)
        if not version or '(*)' in line:
            continue
        version = version[0]
        # get features
        features = line.split(version+' ')[-1]
        if ')' in features: # local path may have ')'
            features = features.split(') ')[-1]
        features = features.split(',')
        # print('line:', line)
        # print('version:', version)
        # print('features:', features)
        if not dependency_trees.get(version):
            dependency_trees[version] = set()
        for feature in features:
            if feature == '':
                continue
            dependency_trees[version].add(feature)
    return dependency_trees



def get_ruf_usage_DB(package_name: str, version: str) -> list:
    '''
    Search ruf usage in the DB given a package version.
    Return a list of ruf usage and needed features, in format [(cond, ruf), ...].
    '''
    cursor = conn.cursor()
    try:
        cursor.execute(f"SELECT conds, feature FROM version_feature INNER JOIN versions_with_name ON version_feature.id = versions_with_name.id WHERE num = '{version}' AND name = '{package_name}' AND feature != 'no_feature_used' ")
        records = cursor.fetchall()
        return records
    except Exception as e:
        print(e)
        return list()



def get_enabled_ruf(features: list, ruf_usage: list) -> list:
    '''
    Given a list of features and ruf usage, return a list of enabled ruf.
    Ruf may only be enabled only when configration predicates are satisfied.
    As different predicates are satisfied in different ways, the rules can be configured here based on how you DEFINE impats.
    '''
    enabled_ruf = set()
    for (cond, ruf) in ruf_usage:
        # 1. Uncond Impact
        if cond == '':
            enabled_ruf.add(ruf)
            continue
        # 2. Cond Impact: Feature
        if len(cond) >= 11 and cond[:11] == 'feature = "':
            feature = cond[11:-1]
            if feature in features:
                print(f'Feature {feature} is enabled through Cond Impact')
                enabled_ruf.add(ruf)
            continue
    return list(enabled_ruf)



def project_dependency_impacts():
    '''
    Run `cargo tree` tool to get the dependency tree of each project.
    Search in our database to get the ruf impacts of each dependency.
    '''

    ruf_usage = dict()
    absolute_output_dir = os.getcwd() + '/' + output_dir
    for dir in os.listdir(absolute_output_dir):
        project_name = dir
        # print('-------------------')
        print(f'{project_name}')
        # print('-------------------')
        target_dir = f'{absolute_output_dir}/{dir}'
        if 'Cargo.toml' not in os.listdir(target_dir):
            print(f'{project_name} does not have Cargo.toml')
            continue
        dependency_trees = get_dependency_tree(f'{target_dir}/Cargo.toml')
        # print(*dependency_trees.items(), sep='\n')
        ruf_impacts = set()
        for (name_ver, features) in dependency_trees.items():
            # print(name_ver, features)
            name_ver = name_ver.split(' ')
            name = name_ver[0]
            ver = name_ver[1][1:]
            usage = get_ruf_usage_DB(name, ver)
            enabled_ruf = get_enabled_ruf(features, usage)
            ruf_impacts.update(enabled_ruf)
            # print(' Usage:', usage)
            # print(' Enabled:', enabled_ruf)
        print(f'{project_name} has {len(ruf_impacts)} ruf impacts: {ruf_impacts}')


def pre_analyze_toml():
    ruf_usage = dict()
    absolute_output_dir = os.getcwd() + '/' + output_dir
    for dir in os.listdir(absolute_output_dir):
        project_name = dir
        target_dir = f'{absolute_output_dir}/{dir}'
        if 'Cargo.toml' not in os.listdir(target_dir):
            print(f'{project_name} does not have Cargo.toml')


def test_dependency_tree():
    dependency_trees = get_dependency_tree('/home/loancold/Projects/Cargo-Ecosystem-Monitor/Cargo-Ecosystem-Monitor/Code/github_ecosystem/projects/lsd/Cargo.toml')
    # print(*dependency_trees.items(), sep='\n')
    for (name_ver, features) in dependency_trees.items():
        print(name_ver, features)
        name_ver = name_ver.split(' ')
        name = name_ver[0]
        ver = name_ver[1][1:]
        usage = get_ruf_usage_DB(name, ver)
        enabled_ruf = get_enabled_ruf(features, usage)
        print(' Usage:', usage)
        print(' Enabled:', enabled_ruf)


def test_reset_all_project_change():
    '''
    Reset all projects to the latest commit before the time.
    '''
    for dir in os.listdir(f'./{output_dir}'):
        print(f'Reset {dir}')
        branch = subprocess.getoutput(f"cd {output_dir}/{dir} && git branch")[2:]
        os.system(f"cd {output_dir}/{dir} && git reset --hard HEAD")


def case_study_projects():
    ruf_usage = dict()
    absolute_output_dir = os.getcwd()
    for dir in case_study_paths:
        project_name = dir
        if dir[0] == '/': # absolute path
            target_dir = dir
        else:
            target_dir = f'{absolute_output_dir}/{dir}'
        print(f'Analyzing {target_dir}')
        files = subprocess.run(['find', target_dir, '-type', 'f', '-name', '*.rs'], stdout=subprocess.PIPE).stdout.decode('utf-8')
        count = 0
        total = len(files.split('\n'))
        for file_name in files.split('\n'):
            count += 1
            print(f'Analyzing {count}/{total}: {file_name}')
            if 'test' in file_name:
                continue
            results = subprocess.run([analyze_path, file_name, '--edition', '2021', '--ruf-analysis'], stdout=subprocess.PIPE).stdout.decode('utf-8')
            re_ori = re.findall("formatori \(\[(.*?)\], (.*?)\)", results)
            re_processed = re.findall("processed \(\[(.*?)\], (.*?)\)", results)
            if len(re_ori) != 0:
                for item in re_ori:
                    cond = item[0]
                    ruf = item[1]
                    if not ruf_usage.get(project_name):
                        ruf_usage[project_name] = set()
                    ruf_usage[project_name].add(ruf)
    for project_name in ruf_usage:
        print(f'{project_name}({len(ruf_usage[project_name])}): {ruf_usage[project_name]}')


import sys

if len(sys.argv) < 2:
    print('Usage: python3 ruf_usage_hot_rust_projects.py [download_hot_projects | reset_all_project_change | ruf_usage | ruf_impacts | case_study]')
    exit()
if sys.argv[1] == 'download_hot_projects':
    url_list = parse_top_projects(target_file_name)
    download_github_projects(url_list)
    checkout_to_time()
elif sys.argv[1] == 'reset_all_project_change':
    test_reset_all_project_change()
elif sys.argv[1] == 'ruf_usage':
    analyze_github_projects()
elif sys.argv[1] == 'ruf_impacts':
    print('Reminder: You need to first correctly configure your crates.io index database before resolving dependencies.')
    project_dependency_impacts()
elif sys.argv[1] == 'case_study':
    case_study_projects()
else:
    print('Invalid command')
    print('Usage: python3 ruf_usage_hot_rust_projects.py [download_hot_projects | reset_all_project_change | ruf_usage | ruf_impacts | case_study]')

