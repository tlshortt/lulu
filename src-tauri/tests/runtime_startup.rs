use tempfile::tempdir;

use tauri_app_lib::db::init_database;

#[test]
fn startup_creates_database_file() {
    let temp_dir = tempdir().expect("tempdir should be created");
    let db_path = temp_dir.path().join("lulu.db");

    let _database = init_database(&db_path).expect("database should initialize on startup");

    assert!(db_path.exists(), "startup should create lulu.db in app data dir");
}

#[test]
fn startup_schema_is_queryable() {
    let temp_dir = tempdir().expect("tempdir should be created");
    let db_path = temp_dir.path().join("lulu.db");

    let database = init_database(&db_path).expect("database should initialize on startup");
    let conn = database.conn.lock().expect("db lock should be available");

    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='sessions'")
        .expect("schema query should prepare");
    let mut rows = stmt.query([]).expect("schema query should run");
    let row =
        rows.next().expect("schema row query should succeed").expect("sessions table should exist");
    let table_name: String = row.get(0).expect("table name should read");

    assert_eq!(table_name, "sessions");
}
