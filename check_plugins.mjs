// bun has built-in sqlite
import { Database } from "bun:sqlite";
import path from "path";
const dbPath = path.join(process.env.LOCALAPPDATA, "Peekoo", "peekoo", "peekoo.sqlite");
const db = new Database(dbPath, { readonly: true });
const rows = db.query("SELECT plugin_key, enabled FROM plugins ORDER BY plugin_key").all();
rows.forEach(r => console.log(r.plugin_key, ":", r.enabled));
db.close();
