use chrono::{DateTime, Utc};
use rusqlite::Connection;

pub fn get_db() -> Connection {
    let conn = Connection::open("sal.db").unwrap();
    return conn;
}

#[derive(Debug)]
struct Log {
    timestamp: DateTime<Utc>,
    id: u64,
}

#[derive(Debug)]
struct Person {
    id: u64,
    username: String,
}
