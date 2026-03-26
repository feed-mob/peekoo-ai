import sqlite3, os, sys
db = os.path.join(os.environ['LOCALAPPDATA'], 'Peekoo', 'peekoo', 'peekoo.sqlite')
print(f"DB: {db}")
conn = sqlite3.connect(db)
cur = conn.cursor()
cur.execute("SELECT plugin_key, enabled FROM plugins ORDER BY plugin_key")
rows = cur.fetchall()
for row in rows:
    print(f"  {row[0]}: enabled={row[1]}")
conn.close()
