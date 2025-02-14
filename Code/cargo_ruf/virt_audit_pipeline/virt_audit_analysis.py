import psycopg2
import re
import logging
import sys
from collections import defaultdict
from thefuzz import fuzz
from typing import Dict, List
from pyparsing import (
    Word,
    nums,
    Suppress,
    Literal,
    QuotedString,
    Group,
    delimitedList,
    Dict as PDict,
    Optional,
)

# 配置日志
logging.basicConfig(
    level=logging.INFO, format="%(asctime)s - %(levelname)s - %(message)s"
)

# 配置数据库连接信息
DB_CONFIG = {
    "dbname": "crates",
    "user": "postgres",
    "password": "postgres",
    "host": "localhost",
    "port": "5432",  # 默认 PostgreSQL 端口
}


# 连接到数据库
def connect_to_db():
    try:
        conn = psycopg2.connect(
            dbname=DB_CONFIG["dbname"],
            user=DB_CONFIG["user"],
            password=DB_CONFIG["password"],
            host=DB_CONFIG["host"],
            port=DB_CONFIG["port"],
        )
        return conn
    except Exception as e:
        logging.error(f"数据库连接失败: {e}")
        sys.exit(1)


def extract_issue_name(output):
    """
    从 output 中提取 issue 名称。假设输出格式为：
    'Issue dep yup-oauth2@0.6.3 is not fixable'
    提取 'yup-oauth2'。
    """
    # 使用正则表达式提取 "Issue dep yup-oauth2@0.6.3 is not fixable" 中的 "yup-oauth2"
    match = re.search(r"Issue dep ([\w\-]+)@[\d\.]+ is not fixable", output)
    if match:
        return match.group(1)  # 返回捕获的 issue 名称
    else:
        return None  # 如果没有匹配到，返回 None


# 基于 name 字段的相似度分类
def classify_name(names, threshold=60):
    """
    根据相似度将名称进行分类。阈值（threshold）设置为80，表示相似度大于等于threshold的名称将被归为同一类。
    """
    # 使用 thefuzz 的 process.extractOne 来将名称进行分类
    groups = defaultdict(list)
    for name in names:
        found = False
        # 查找与现有组的名称相似度大于阈值的组
        for key in list(groups.keys()):
            if fuzz.ratio(name, key) >= threshold:
                groups[key].append(name)
                found = True
                break
        # 如果没有找到匹配的组，则新建一个组
        if not found:
            groups[name].append(name)

    return groups


# 查询并处理数据库中的数据
def process_fix_fail_data():
    # 连接到数据库
    conn = connect_to_db()
    cursor = conn.cursor()

    # 初始化统计数据结构
    issue_counts = defaultdict(int)  # 记录每个 issue 名称的出现次数
    names = []  # 用于存储所有的 name 字段，后续进行相似度分类

    try:
        # 查询 result = 'fix fail' 的数据
        query = """
            SELECT version_id, output, name
            FROM virt_analysis
            WHERE result = 'fix fail'
        """
        cursor.execute(query)
        rows = cursor.fetchall()

        # 遍历查询结果并提取统计数据
        for row in rows:
            version_id, output, name = row

            # 统计 issue 名称
            issue_name = extract_issue_name(output)
            if issue_name:
                issue_counts[issue_name] += 1

            # 将 name 字段添加到分类列表
            names.append(name)

        # 对 name 字段进行相似度分类
        name_groups = classify_name(names)  # 确保在这里调用 classify_name

        # 打印 issue 名称统计结果，仅显示 count > 100 的 issue，并按 count 降序排列
        logging.info("=" * 50)
        logging.info("Issue 名称统计结果 (仅显示 count > 100, 按降序排列):")
        logging.info("=" * 50)

        # 过滤并按 count 降序排列
        sorted_issues = sorted(issue_counts.items(), key=lambda x: x[1], reverse=True)
        for issue_name, count in sorted_issues:
            if count > 100:
                logging.info(f"Issue: {issue_name}, Count: {count}")

        logging.info("=" * 50)

        # 打印 name 相似度分类结果，仅显示 count > 100 的分组，并按 count 降序排列
        logging.info("=" * 50)
        logging.info("Name 相似度分类结果 (仅显示 count > 100 的分组, 按降序排列):")
        logging.info("=" * 50)

        # 过滤并按 count 降序排列
        sorted_name_groups = sorted(
            name_groups.items(), key=lambda x: len(x[1]), reverse=True
        )
        for group, names in sorted_name_groups:
            if len(names) > 100:  # 如果该分组中的名称数量大于 100，才输出
                logging.info(f"Group: {group}, Count: {len(names)}")
                logging.info("-" * 50)

        logging.info("=" * 50)

    except Exception as e:
        logging.error(f"查询或处理数据时发生错误: {e}")
    finally:
        cursor.close()
        conn.close()


def parse_summary(summary_str: str) -> Dict:
    """
    解析 Summary 字符串，提取 fix_rustv 和 fix_deps 信息。
    使用 pyparsing 进行可靠的解析。
    """
    # 定义基本元素
    LBRACE, RBRACE, LBRACKET, RBRACKET, LPAREN, RPAREN, COLON, COMMA = map(
        Suppress, "{}[]():,"
    )

    integer = Word(nums).setParseAction(lambda t: int(t[0]))
    identifier = QuotedString('"')
    feature = QuotedString('"')

    # 定义 feature list
    feature_list = Group(LBRACKET + Optional(delimitedList(feature)) + RBRACKET)

    # 定义修复条目
    fix_entry = Group(
        LPAREN
        + identifier("package_name")
        + COMMA
        + QuotedString('"')("from_version")
        + COMMA
        + QuotedString('"')("to_version")
        + COMMA
        + feature_list("features")
        + RPAREN
    )

    # 定义依赖项
    dep_entry = Group(
        identifier("dep_name")
        + COLON
        + LBRACKET
        + Optional(delimitedList(fix_entry))("fix_entries")
        + RBRACKET
    )

    # 使用 delimitedList 处理多个 dep_entry 之间的逗号分隔
    fix_deps = PDict(delimitedList(dep_entry))

    # 定义 Summary 的整体结构
    summary_parser = (
        Literal("Summary")
        + LBRACE
        + Literal("fix_rustv")
        + COLON
        + integer("fix_rustv")
        + COMMA
        + Literal("fix_deps")
        + COLON
        + LBRACE
        + fix_deps("fix_deps")
        + RBRACE
        + RBRACE
    )

    try:
        parsed = summary_parser.parseString(summary_str, parseAll=True)
    except Exception as e:
        logging.error(f"解析 Summary 失败: {e}")
        raise ValueError(f"无法解析 Summary 字符串: {summary_str}") from e

    fix_rustv = parsed.fix_rustv
    fix_deps_dict = {}

    for dep_name, fix_entries in parsed.fix_deps.items():
        fix_list = []
        for entry in fix_entries:

            fix_entry_cont = (
                entry.package_name,
                entry.from_version,
                entry.to_version,
                entry.features.asList(),
            )

            fix_list.append(fix_entry_cont)
        fix_deps_dict[dep_name] = fix_list

    if not fix_deps_dict:
        raise ValueError(f"fix_deps 不能为空: {summary_str}")

    summary_data = {"fix_rustv": fix_rustv, "fix_deps": fix_deps_dict}

    return summary_data


# 插入 fix 详情到 virt_fix_details 表
def insert_fix_detail(
    cursor,
    version_id: int,
    dep_name: str,
    old_version: str,
    new_version: str,
    upfix: int,
    status: str,
):
    insert_query = """
        INSERT INTO virt_fix_details (version_id, issue_dep, old, new, upfix, status)
        VALUES (%s, %s, %s, %s, %s, %s)
    """
    cursor.execute(
        insert_query, (version_id, dep_name, old_version, new_version, upfix, status)
    )
    logging.info(
        f"插入 fix 详情: version_id={version_id}, dep_name={dep_name}, old={old_version}, new={new_version}, upfix={upfix}, status={status}"
    )


# 处理 features 并判断稳定性，cursor 作为外部输入
def process_features(cursor, features: List[str]) -> str:
    stable = True  # 假设是 stable，若有不稳定状态则改为 unstable

    for feature in features:
        cursor.execute("SELECT status FROM feature_status WHERE name = %s", (feature,))
        result = cursor.fetchone()

        if result:
            status = result[0]
            if status in ["removed", "unknown"]:
                raise ValueError(f"Feature '{feature}' 状态不可用，状态: {status}")
            elif status == "accepted":
                continue  # accepted 是 stable
            elif status in ["active", "incomplete"]:
                stable = False  # active 和 incomplete 表示 unstable
            else:
                logging.warning(
                    f"Feature '{feature}' 的状态 '{status}' 未定义，默认为 unknown"
                )
                raise ValueError(f"Feature '{feature}' 状态未定义，状态: {status}")
        else:
            logging.error(f"Feature '{feature}' 不存在于数据库中")
            raise ValueError(f"Feature '{feature}' 不存在")

    return "Stable" if stable else "Unstable"


def process_fix_success_data():
    # 连接到数据库
    conn = connect_to_db()
    cursor = conn.cursor()

    # 创建 virt_fix_details 表（如果尚未创建）
    create_table_query = """
    CREATE TABLE IF NOT EXISTS virt_fix_details (
        version_id INT,
        issue_dep VARCHAR,
        old VARCHAR,
        new VARCHAR,
        upfix INT,
        status VARCHAR
    );
    """
    cursor.execute(create_table_query)
    conn.commit()

    query = """
        SELECT version_id, summary FROM virt_analysis
        WHERE result = 'success'
    """
    cursor.execute(query)
    rows = cursor.fetchall()

    for row in rows:
        version_id = row[0]  # 获取 version_id
        summary_str = row[1]  # 获取 summary 字段内容
        try:
            summary_data = parse_summary(summary_str)
        except ValueError as e:
            logging.error(f"解析错误: {e}")
            raise  # 停止执行

        logging.info(f"正在处理 version_id={version_id}")
        fix_deps = summary_data["fix_deps"]
        for dep_name_ver, dep_list in fix_deps.items():
            if not dep_list:
                error_msg = f"fix_deps 中的依赖项列表为空: version_id={version_id}, dep_name_ver={dep_name_ver}"
                logging.error(error_msg)
                raise ValueError(error_msg)

            upfixes = 0
            downfix = None

            # 遍历 dep_list
            for i, dep_entry in enumerate(dep_list):
                package_name, old_version, new_version, feats = dep_entry

                if package_name != dep_name_ver.split("@")[0]:
                    # 记作一次 upfix
                    upfixes += 1
                else:
                    # 记作一次 downfix
                    if i != len(dep_list) - 1:
                        # 如果不是最后一个元素，报错并停止
                        error_msg = f"Expected last element in the list for downfix, but found earlier one: version_id={version_id}, dep_name={dep_name}, package_name={package_name}"
                        logging.error(error_msg)
                        raise ValueError(error_msg)

                    status = process_features(cursor, feats)

                    downfix = (
                        version_id,
                        dep_name_ver.split("@")[0],
                        old_version,
                        new_version,
                        upfixes,
                        status,
                    )

            if downfix:
                insert_fix_detail(cursor, *downfix)
            else:
                insert_fix_detail(
                    cursor,
                    version_id,
                    dep_name_ver.split("@")[0],
                    dep_name_ver.split("@")[1],
                    "remove",
                    upfixes,
                    "Stable",
                )
        # 提交所有插入操作
    try:
        conn.commit()
        logging.info("所有 fix 详情已成功插入并提交。")
    except Exception as e:
        conn.rollback()
        logging.error(f"提交事务失败: {e}")
        raise


if __name__ == "__main__":
    process_fix_success_data()
