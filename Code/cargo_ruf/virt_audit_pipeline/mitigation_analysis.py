import psycopg2
import sys
import logging

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')

DB_CONFIG = {
    'dbname': 'crates',
    'user': 'postgres',
    'password': 'postgres',
    'host': 'localhost',
    'port': '5432'
}

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

def create_mitigation_table(conn):
    cursor = conn.cursor()
    create_table_query = """
    CREATE TABLE IF NOT EXISTS mitigation_table (
        version_id  INTEGER PRIMARY KEY,
        before_fix  VARCHAR,
        after_deptree_fix VARCHAR,
        after_direct_fix  VARCHAR,
        after_rustc_fix   VARCHAR,
        fixable     BOOLEAN
    )
    """
    cursor.execute(create_table_query)
    conn.commit()
    cursor.close()

def fetch_versions(conn):
    cursor = conn.cursor()
    cursor.execute("SELECT version_id FROM rustc_audit_results WHERE before_mitigation = 'failure'")
    return cursor.fetchall()

def fetch_mitigation_data(conn, version_id):
    
    pass

# def fetch_mitigation_data(conn, version_id):
#     # Check before fix
#     cursor = conn.cursor()
#     cursor.execute("SELECT before_mitigation FROM rustc_audit_results WHERE version_id = %s", (version_id,))
#     sql_res = cursor.fetchone()
#     assert(sql_res is not None)
    
#     before_fix = sql_res[0]
#     if before_fix != 'failure':
#         return before_fix, before_fix, before_fix, before_fix, True

#     # Check deptree_fix
#     cursor.execute("SELECT result FROM virt_analysis WHERE version_id = %s", (version_id,))
#     sql_res = cursor.fetchone()
#     if sql_res is not None:
#         # It's not direct usage
#         after_deptree_fix = sql_res[0]
#         if after_deptree_fix == 'success':
#             cursor.execute("SELECT stability FROM virt_fix_table WHERE version_id = %s", (version_id,))
#             sql_res = cursor.fetchall()
#             if sql_res is None:
#                 # It may be pure upfix, we assume it's stable
#                 after_deptree_fix = 'stable'
#             elif len(sql_res) != 1:
#                 # Multiple fix, choose the worst one.
#                 after_deptree_fix = 'stable'
#                 for fix in sql_res:
#                     if fix == 'unstable':
#                         after_deptree_fix = 'unstable'
#                         break
#             else:
#                 after_deptree_fix = sql_res[0]
#         else:
#             after_deptree_fix = 'failure'
#     else:
#         # Or it need direct usage
#         after_deptree_fix = 'failure'
    
#     if after_deptree_fix != 'failure':
#         return before_fix, after_deptree_fix, after_deptree_fix, after_deptree_fix, True
    
#     # Check direct_fix
#     cursor.execute("SELECT fix_status FROM direct_audit_result WHERE version_id = %s", (version_id,))
#     sql_res = cursor.fetchone()
#     if sql_res is None:
#         # Then it's a indirect usage, use the previous result
#         after_direct_fix = after_deptree_fix
#     else:
#         after_direct_fix = sql_res[0]
    
#     if after_direct_fix != 'failure':
#         return before_fix, after_deptree_fix, after_direct_fix, after_direct_fix, True
    
#     # Check rustc_fix
#     cursor.execute("SELECT after_mitigation FROM rustc_audit_results WHERE version_id = %s", (version_id,))
#     sql_res = cursor.fetchone()
#     assert(sql_res is not None)
    
#     after_rustc_fix = sql_res[0]
#     if after_rustc_fix == 'failure':
#         fixable = False
#     else:
#         fixable = True

#     cursor.close()
#     return before_fix, after_deptree_fix, after_direct_fix, after_rustc_fix, fixable
    

def main():
    conn = connect_to_db()
    
    try:
        # Create table if not exists
        create_mitigation_table(conn)
        versions = fetch_versions(conn)
        cursor = conn.cursor()
        
        for version in versions:
            version_id = version[0]
            try:
                print(f"处理 version_id: {version_id}")
                before_fix, after_deptree_fix, after_direct_fix, after_rustc_fix, fixable = fetch_mitigation_data(conn, version_id)
            except Exception as e:
                logging.error(f"处理 version_id: {version_id} 时出错: {e}")
                raise e
            
            cursor.execute("INSERT INTO mitigation_table (version_id, before_fix, after_deptree_fix, after_direct_fix, after_rustc_fix, fixable) VALUES (%s, %s, %s, %s, %s, %s)", (version_id, before_fix, after_deptree_fix, after_direct_fix, after_rustc_fix, fixable))

        conn.commit()        
    finally:
        conn.close()
        
if __name__ == "__main__":
    main()
