use chrono::DateTime;
use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::io;

use crate::models::get_db;

pub fn migrate() -> io::Result<()> {
    let conn = get_db();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS logs (
            id    INTEGER,
            timestamp  TEXT NOT NULL,
            PRIMARY KEY (id, timestamp)
        )",
        (), // empty list of parameters.
    )
    .unwrap();

    conn.execute(
        "CREATE INDEX logs_timestamp_idx
            ON logs (timestamp)",
        (), // empty list of parameters.
    )
    .unwrap();

    conn.execute(
        "CREATE INDEX logs_id_idx
            ON logs (id)",
        (), // empty list of parameters.
    )
    .unwrap();

    for file in fs::read_dir("logs").expect("logs dir to exist") {
        let file = file.unwrap();
        let path = file.path();
        if path.is_file() {
            let mut rdr = csv::Reader::from_path(path).unwrap();
            for line in rdr.records() {
                let parts = line.unwrap();
                let timestamp = DateTime::parse_from_rfc3339(&parts[0]).unwrap();
                let userid: u64 = parts[1].parse().unwrap();
                let res = conn.execute(
                    "INSERT INTO logs (id, timestamp) VALUES (?1, ?2)",
                    (&userid, timestamp),
                );
                match res {
                    Ok(_) => (),
                    Err(err) => println!("{err:?}"),
                }
            }
        }
    }

    conn.execute(
        "CREATE TABLE IF NOT EXISTS people (
            id    INTEGER PRIMARY KEY,
            username  TEXT NOT NULL
        )",
        (), // empty list of parameters.
    )
    .unwrap();

    conn.execute(
        "CREATE INDEX people_id_idx
            ON people(id)",
        (), // empty list of parameters.
    )
    .unwrap();

    let json_file = fs::read_to_string("users.json").unwrap();
    let users = json::parse(&json_file).unwrap();

    for (userid, username) in users.entries() {
        let userid: u64 = userid.parse().unwrap();
        let username = username.as_str().unwrap();

        let res = conn.execute(
            "INSERT INTO people (id, username) VALUES (?1, ?2)",
            (userid, username),
        );
        match res {
            Ok(_) => (),
            Err(err) => println!("{err:?}"),
        }
    }

    Ok(())
}

pub fn dump() -> io::Result<()> {
    let conn = get_db();

    let mut logs_stmt = conn
        .prepare("SELECT id, timestamp FROM logs ORDER BY timestamp ASC")
        .unwrap();

    let mut days = HashMap::new();
    let logs_res = logs_stmt
        .query_map([], |row| {
            let id: u64 = row.get(0).unwrap();
            let timestamp: DateTime<Utc> = row.get(1).unwrap();
            Ok((id, timestamp))
        })
        .unwrap();
    for row in logs_res {
        let (id, timestamp) = row.unwrap();
        let date = timestamp.format("%Y%m%d").to_string();
        days.entry(date)
            .or_insert_with(Vec::new)
            .push((timestamp, id));
    }

    fs::create_dir_all("logs").unwrap();
    for (date, entries) in days {
        let mut writer = csv::Writer::from_path(format!("logs/{date}.log")).unwrap();
        for (timestamp, id) in entries {
            writer
                .write_record(&[timestamp.to_rfc3339(), id.to_string()])
                .unwrap();
        }
    }

    let mut people_stmt = conn.prepare("SELECT id, username FROM people").unwrap();

    let user_map: HashMap<String, String> = people_stmt
        .query_map([], |row| {
            let id: u64 = row.get(0).unwrap();
            let username: String = row.get(1).unwrap();
            Ok((id.to_string(), username))
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    let dumped = json::stringify_pretty(user_map, 2);
    fs::write("users.json", dumped).unwrap();

    Ok(())
}
