import psycopg2
import re
import json
import sys
import logging

# 配置日志
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')

# 配置数据库连接信息
DB_CONFIG = {
    'dbname': 'crates',
    'user': 'postgres',
    'password': 'postgres',
    'host': 'localhost',
    'port': '5432'  # 默认 PostgreSQL 端口
}

# 连接到数据库
def connect_to_db():
    try:
        conn = psycopg2.connect(
            dbname=DB_CONFIG['dbname'],
            user=DB_CONFIG['user'],
            password=DB_CONFIG['password'],
            host=DB_CONFIG['host'],
            port=DB_CONFIG['port']
        )
        return conn
    except Exception as e:
        logging.error(f"数据库连接失败: {e}")
        sys.exit(1)

# 创建目标表（如果不存在）
def create_fix_table(conn):
    cursor = conn.cursor()
    create_table_query = """
    CREATE TABLE IF NOT EXISTS virt_fix_table (
        version_id INTEGER NOT NULL,
        package TEXT NOT NULL,
        old_version TEXT NOT NULL,
        old_features JSONB,
        new_version TEXT NOT NULL,
        new_features JSONB,
        stability TEXT NOT NULL,
        UNIQUE (version_id, package, old_version, new_version)
    );
    """
    cursor.execute(create_table_query)
    conn.commit()
    cursor.close()

# 从数据库读取输出数据
def fetch_outputs(conn):
    cursor = conn.cursor()
    cursor.execute("SELECT version_id, output FROM virt_analysis WHERE result = 'success'")
    return cursor.fetchall()

# 获取 feature 的状态
def get_feature_status(conn, features):
    cursor = conn.cursor()
    status_mapping = {}
    for feature in features:
        cursor.execute("SELECT status FROM feature_status WHERE name = %s", (feature,))
        result = cursor.fetchone()
        if result:
            status_mapping[feature] = result[0]
        else:
            cursor.close()
            raise ValueError(f"Feature '{feature}' 未在 feature_status 表中找到状态。")
    cursor.close()
    return status_mapping

# 确定整体稳定性
def determine_stability(status_mapping):
    unstable_statuses = {"incomplete", "active"}
    for status in status_mapping.values():
        if status in unstable_statuses:
            return "unstable"
    return "stable"

# 提取修复信息
def parse_fix_info(output, conn):
    fix_info = []  # 使用列表来记录所有修复记录
    lines = output.splitlines()

    # 更新后的正则表达式，支持带有后缀的版本号
    package_pattern = re.compile(r"check_fix: checking ruf enabled package ([\w\-_]+)@([\d]+\.[\d]+\.[\d]+(?:-[\w\.]+)?) rufs: (\[.*?\])")
    fix_pattern = re.compile(r"check_fix: Try fixing issue dep ([\w\-_]+)@([\d]+\.[\d]+\.[\d]+(?:-[\w\.]+)?) -> ([\d]+\.[\d]+\.[\d]+(?:-[\w\.]+)?)")

    # 当前包的状态
    current_packages = {}

    for i, line in enumerate(lines):
        # 检查包的信息
        match_package = package_pattern.search(line)
        if match_package:
            package_name = match_package.group(1)
            version = match_package.group(2)
            rufs = match_package.group(3)
            try:
                features = json.loads(rufs)
            except json.JSONDecodeError:
                raise ValueError(f"无法解析 rufs: {rufs} in line: {line}")
            # 更新当前包的状态
            current_packages[package_name] = {
                'version': version,
                'features': features
            }

        # 检查修复操作
        match_fix = fix_pattern.search(line)
        if match_fix:
            package = match_fix.group(1)
            old_version = match_fix.group(2)
            new_version = match_fix.group(3)

            # 获取旧的 features
            if package not in current_packages:
                raise ValueError(f"修复操作中包 '{package}' 未在之前的包检查中找到。")
            old_features = current_packages[package].get('features', [])

            # 查找新版本的 features
            new_features = []
            # 搜索后续行，直到找到新的 package check
            for subsequent_line in lines[i+1:]:
                match_new_package = package_pattern.search(subsequent_line)
                if match_new_package:
                    new_pkg_name = match_new_package.group(1)
                    new_pkg_version = match_new_package.group(2)
                    if new_pkg_name == package and new_pkg_version == new_version:
                        try:
                            new_features = json.loads(match_new_package.group(3))
                        except json.JSONDecodeError:
                            raise ValueError(f"无法解析新版本 rufs: {match_new_package.group(3)} for package {package} in line: {subsequent_line}")
                        break
            else:
                # 如果没有找到新的 package check，假设 new_features 为 []
                logging.info(f"未找到包 '{package}' 的新版本 '{new_version}' 的特性信息，认为 new_features 为 []")
                new_features = []

            # 获取 feature 的状态
            if new_features:
                status_mapping = get_feature_status(conn, new_features)
                stability = determine_stability(status_mapping)
            else:
                # 如果 new_features 为空，稳定性为 stable
                stability = "stable"

            # 记录修复信息
            fix_record = {
                "Package": package,
                "Old Version": old_version,
                "Old Features": old_features,
                "New Version": new_version,
                "New Features": new_features,
                "Stability": stability
            }
            fix_info.append(fix_record)

            # 更新当前包的状态为新版本
            current_packages[package] = {
                'version': new_version,
                'features': new_features
            }

    return fix_info

# 将修复信息插入到数据库
def insert_fix_info(conn, fix_info, version_id):
    cursor = conn.cursor()

    for info in fix_info:
        try:
            cursor.execute("""
                INSERT INTO virt_fix_table (version_id, package, old_version, old_features, new_version, new_features, stability)
                VALUES (%s, %s, %s, %s, %s, %s, %s)
            """, (
                version_id,
                info['Package'],
                info['Old Version'],
                json.dumps(info['Old Features']),
                info['New Version'],
                json.dumps(info['New Features']),
                info['Stability']
            ))
            logging.info(f"插入记录: {info['Package']} {info['Old Version']} -> {info['New Version']}, Stability: {info['Stability']}")
        except psycopg2.errors.UniqueViolation:
            # 如果记录已存在，则跳过插入
            logging.warning(f"记录已存在 (version_id: {version_id}, package: {info['Package']}, old_version: {info['Old Version']}, new_version: {info['New Version']})")
            conn.rollback()
        except Exception as e:
            # 任何其他异常，抛出错误并停止脚本
            conn.rollback()
            cursor.close()
            raise RuntimeError(f"插入记录失败 (version_id: {version_id}, package: {info['Package']}): {e}")

    conn.commit()
    cursor.close()

# 主函数
def main():
    conn = connect_to_db()

    try:
        # 创建目标表（如果不存在）
        create_fix_table(conn)

        # 读取 output 表中的数据
        outputs = fetch_outputs(conn)

        for version_id, output in outputs:
            try:
                fix_info = parse_fix_info(output, conn)
                if fix_info:
                    insert_fix_info(conn, fix_info, version_id)
                    logging.info(f"已处理 version_id: {version_id}，修复记录数: {len(fix_info)}")
                else:
                    logging.info(f"没有修复记录 version_id: {version_id}")
            except Exception as e:
                # 在遇到错误时，打印错误并停止脚本
                logging.error(f"处理 version_id: {version_id} 时出错: {e}")
                raise e
    finally:
        conn.close()

if __name__ == "__main__":
    main()
