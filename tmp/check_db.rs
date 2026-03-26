use rusqlite::Connection;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let local_app_data = std::env::var("LOCALAPPDATA")?;
    let db_path = PathBuf::from(local_app_data)
        .join("Peekoo")
        .join("peekoo")
        .join("peekoo.sqlite");

    if !db_path.exists() {
        println!("Database not found at {:?}", db_path);
        return Ok(());
    }

    let conn = Connection::open(db_path)?;
    println!("Database opened successfully.");

    let mut stmt = conn.prepare("SELECT plugin_key, enabled, version FROM plugins")?;
    let plugin_iter = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, bool>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    println!("Plugins in database:");
    for plugin in plugin_iter {
        let (key, enabled, version) = plugin?;
        println!("- Key: {}, Enabled: {}, Version: {}", key, enabled, version);
    }

    Ok(())
}
