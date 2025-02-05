use chrono::{DateTime, TimeDelta, Utc};
use ratatui::{
    style::Stylize,
    text::Span,
};
use rusqlite::Connection;

pub fn get_db() -> Connection {
    
    Connection::open("sal.db").unwrap()
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
    pub stats: Stats,
}

#[derive(Debug, Clone, Copy)]
pub struct Stats {
    pub longest_day: Day,
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

        let stats = Stats::load_for_user(uid);

        Self {
            id: uid,
            username,
            stats,
        }
    }
}

impl Stats {
    fn load_for_user(uid: u64) -> Self {
        let conn = get_db();

        let days = get_days(uid, conn);
        let longest_day = get_longest_day(&days);

        Self { longest_day }
    }
}

fn get_days(uid: u64, conn: Connection) -> Vec<Day> {
    let query = "
    SELECT
        DATE(timestamp) AS day,
        MIN(timestamp) AS first_timestamp,
        MAX(timestamp) AS last_timestamp,
        JULIANDAY(MAX(timestamp)) - JULIANDAY(MIN(timestamp)) AS difference_in_days
    FROM
        logs
    WHERE
        id = (?1)
    GROUP BY
        DATE(timestamp)
    ";

    let mut stmt = conn.prepare(query).unwrap();

    let days = stmt
        .query_map([uid], |row| {
            let start: DateTime<Utc> = row.get(1).unwrap();
            let end: DateTime<Utc> = row.get(2).unwrap();

            Ok(Day::new(start, end))
        })
        .unwrap();
    let days: Result<Vec<_>, _> = days.collect();
    days.unwrap()
}

fn get_longest_day(days: &[Day]) -> Day {
    *days.iter().max_by_key(|day| day.span()).unwrap()
}

#[derive(Debug, Clone, Copy)]
pub struct Day {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl Day {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self { start, end }
    }

    pub fn to_string(&self) -> Vec<Span<'_>> {
        let diff = self.end - self.start;
        let diff_formatted = format!(
            "{} timer og {} minutter",
            diff.num_hours(),
            diff.num_minutes() % 60
        );
        vec![
            "Lengste dag: ".into(),
            self.start.format("%d/%m").to_string().yellow(),
            ". Fra ".into(),
            self.start.format("%H:%M").to_string().yellow(),
            " til: ".into(),
            self.end.format("%H:%M").to_string().yellow(),
            ". Det er hele ".into(),
            diff_formatted.green(),
        ]
    }

    pub fn span(&self) -> TimeDelta {
        self.end - self.start
    }
}
