//! # The module that initializes and optionally drops a database.
use super::*;

/// The API used by the `domain` to drop a database.
/// 
/// The SQL to drop the database schema is stored in `sql/schema_drop.sql` within the source directory.
/// 
/// # Arguments
/// 
/// * `conn` - a connection to the database.
/// * `reclaim_space` - when `true` space used by an existing database will be reclaimed.
pub fn drop(conn: &sql::Connection, reclaim_space: bool) -> Result<()> {
    let drop_sql = include_str!("sql/schema_drop.sql");
    log::debug!("drop schema");
    conn.execute_batch(drop_sql)?;
    if reclaim_space {
        conn.execute("VACUUM", ())?;
    }
    Ok(())
}

/// The API used by the `domain` to initialize a database.
/// 
/// The SQL to drop the database schema is stored in `sql/schema_init.sql` within the source directory.
/// 
/// # Arguments
/// 
/// * `conn` - a connection to the database.
pub fn init(conn: &sql::Connection) -> Result<()> {
    let schema_sql = include_str!("sql/schema_init.sql");
    log::debug!("init schema");
    conn.execute_batch(schema_sql)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    #[test]
    fn create() {
        let conn = super::database_connection(Some(&PathBuf::from("test.db"))).unwrap();
        super::drop(&conn, true).unwrap();
        super::init(&conn).unwrap();
    }
}
