from glob import glob
import os
import sys
import re
import subprocess
import json

target_file_name = 'rust_hot_projects_20220811.md'
output_dir = './projects'
time = '2022-08-11 00:00:00 +0000'
analyze_path = '/home/loancold/Projects/Cargo-Ecosystem-Monitor/Cargo-Ecosystem-Monitor/rust/build/x86_64-unknown-linux-gnu/stage1/bin/rustc' # The path of our ruf usage analyzer (modified rustc)


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


def project_dependency_impacts():
    '''
    Run `cargo tree` tool to get the dependency tree of each project.
    Search in our database to get the ruf impacts of each dependency.
    '''

    ruf_usage = dict()
    absolute_output_dir = os.getcwd() + '/' + output_dir
    for dir in os.listdir(absolute_output_dir):
        project_name = dir
        print(f'Analyzing {project_name}')
        target_dir = f'{absolute_output_dir}/{dir}'
        if 'Cargo.toml' not in os.listdir(target_dir):
            print(f'{project_name} does not have Cargo.toml')
            continue
        results = subprocess.run(['cargo', 'metadata', '--manifest-path', f'{target_dir}/Cargo.toml', '-e', 'no-dev', '--all-features', '--target', 'all'], stdout=subprocess.PIPE).stdout.decode('utf-8')
        metadata = json.loads(results)
        for package in metadata['packages']:
        versions = re.findall("[\w-]+ v[0-9]+.[0-9]+.[0-9]+[\S]*", results)
        version_set = set()
        for verion in versions:
            version_set.add(verion)
        print(*version_set, sep='\n')
        # for version in versions:
        #     name_ver = version.split(' ')
        #     name = name_ver[0]
        #     ver = name_ver[1][1:] # remove the first 'v'
        #     print(name, ver)


def pre_analyze_toml():
    ruf_usage = dict()
    absolute_output_dir = os.getcwd() + '/' + output_dir
    for dir in os.listdir(absolute_output_dir):
        project_name = dir
        target_dir = f'{absolute_output_dir}/{dir}'
        if 'Cargo.toml' not in os.listdir(target_dir):
            print(f'{project_name} does not have Cargo.toml')



# url_list = parse_top_projects(target_file_name)
download_github_projects(url_list)
checkout_to_time()
analyze_github_projects()
# pre_analyze_toml()
# project_dependency_impacts()