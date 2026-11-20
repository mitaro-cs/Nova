use rusqlite::{Connection, backup::Backup};
use std::{env, error::Error, time::Duration};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let source = args.next().ok_or("usage: nova-sqlite-backup SOURCE DESTINATION")?;
    let destination = args.next().ok_or("usage: nova-sqlite-backup SOURCE DESTINATION")?;
    let from = Connection::open(source)?;
    let mut to = Connection::open(destination)?;
    {
        let backup = Backup::new(&from, &mut to)?;
        backup.run_to_completion(128, Duration::from_millis(20), None)?;
    }
    let integrity: String = to.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(format!("database integrity check failed: {integrity}").into());
    }
    println!("SQLite snapshot written and integrity-checked.");
    Ok(())
}
