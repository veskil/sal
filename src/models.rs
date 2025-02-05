use chrono::{DateTime, TimeDelta, Utc};
use rusqlite::Connection;

pub fn get_db() -> Connection {
    let conn = Connection::open("sal.db").unwrap();
    return conn;
}

/// Tablename `logs`
#[derive(Debug)]
pub struct Log {
    timestamp: DateTime<Utc>,
    id: u64,
}

/// Tablename `people`
#[derive(Debug)]
pub struct Person {
    pub id: u64,
    pub username: String,
}

#[derive(Debug, Clone, Copy)]
pub struct Stats {
    pub longest_day: TimeDelta,
}

impl Person {
    pub fn load(uid: u64) -> Self {
        let conn = get_db();
        let username = conn
            .query_row("SELECT username FROM people WHERE id=($1)", [uid], |row| {
                let un = row.get(0).unwrap_or_else(|uid| uid.to_string());
                Ok(un)
            })
            .unwrap();

        Self {
            id: uid,
            username: username,
        }
    }

    pub fn get_stats(&self) -> Stats {
        let conn = get_db();

        let longest_day = self.get_longest_day(conn);

        Stats { longest_day }
    }

    fn get_longest_day(&self, conn: Connection) -> TimeDelta {
        let query = "
        SELECT
            DATE(timestamp) AS day,
            MIN(timestamp) AS first_timestamp,
            MAX(timestamp) AS last_timestamp,
            JULIANDAY(MAX(timestamp)) - JULIANDAY(MIN(timestamp)) AS difference_in_days
        FROM
            logs
        GROUP BY
            DATE(timestamp)
        ORDER BY
            difference_in_days DESC";

        let longest_day = conn
            .query_row(&query, [], |row| {
                let ld: f64 = row.get(3).unwrap();
                let ld_seconds = ld * 24.0 * 60.0 * 60.0;
                eprintln!("{ld_seconds}");
                let delta = TimeDelta::seconds(ld_seconds as i64);
                Ok(delta)
            })
            .unwrap();

        return longest_day;
    }
}
