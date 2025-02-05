use chrono::{DateTime, Datelike, TimeDelta, Utc, Weekday};
use chrono_tz::{Europe::Oslo, Tz};
use itertools::Itertools;
use ratatui::{style::Stylize, text::Span};
use rusqlite::Connection;

pub fn get_db() -> Connection {
    Connection::open("sal.db").unwrap()
}

/// Tablename `logs`
#[allow(unused)]
#[derive(Debug)]
pub struct Log {
    timestamp: DateTime<Utc>,
    id: u64,
}

/// Tablename `people`
#[allow(unused)]
#[derive(Debug)]
pub struct Person {
    pub id: u64,
    pub id2: Option<u64>,
    pub username: String,
    pub stats: Stats,
}

impl Person {
    pub fn load(uid: u64) -> Self {
        let conn = get_db();
        let username = conn
            .query_row("SELECT username FROM people WHERE id=($1)", (uid,), |row| {
                let un = row.get(0).unwrap();
                Ok(un)
            })
            .unwrap_or_else(|uid| uid.to_string());

        let id2: Option<u64> = conn
            .query_row(
                "SELECT id FROM people WHERE USERNAME=($1) AND id!=($2)",
                (&username, uid),
                |row| {
                    let un = row.get(0).unwrap();
                    Ok(un)
                },
            )
            .unwrap_or(None);

        let stats = Stats::load_for_user(uid, id2);

        Self {
            id: uid,
            id2,
            username,
            stats,
        }
    }

    pub fn register(uid: u64) {
        let conn = get_db();
        conn.execute(
            "INSERT INTO logs (id, timestamp) VALUES (?1, ?2)",
            (uid, Utc::now()),
        )
        .unwrap();
    }
}

#[derive(Debug)]
pub struct Stats {
    pub longest_day: Day,
    pub streak: usize,
}

impl Stats {
    fn load_for_user(uid: u64, uid2: Option<u64>) -> Self {
        let conn = get_db();

        let days = get_days(uid, uid2, conn);
        let longest_day = get_longest_day(&days);
        let streak = get_streak(&days);

        Self {
            longest_day,
            streak,
        }
    }
}

fn get_days(uid: u64, uid2: Option<u64>, conn: Connection) -> Vec<Day> {
    let query = "
    SELECT
        DATE(timestamp) AS day,
        MIN(timestamp) AS first_timestamp,
        MAX(timestamp) AS last_timestamp,
        JULIANDAY(MAX(timestamp)) - JULIANDAY(MIN(timestamp)) AS difference_in_days
    FROM
        logs
    WHERE
        id IN (?1, ?2)
    GROUP BY
        DATE(timestamp, '-5 hours', 'localtime')
    ORDER BY
        day DESC
    ";

    let mut stmt = conn.prepare(query).unwrap();

    let days = stmt
        .query_map([uid, uid2.unwrap_or(uid)], |row| {
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

fn get_streak(days: &[Day]) -> usize {
    let mut streak = 1;

    // (today, yesterday), (yesterday, yesyesterday) etc
    for (day, prev_day) in days.iter().tuple_windows() {
        let day = day.end - TimeDelta::hours(5);
        let prev_day = prev_day.end - TimeDelta::hours(5);
        if day.date_naive() == prev_day.date_naive() {
            continue;
        }

        let mut btwn = day - TimeDelta::days(1);
        let mut streak_good = true;
        while btwn.date_naive() != prev_day.date_naive() {
            if !(btwn.weekday() == Weekday::Sat || btwn.weekday() == Weekday::Sun) {
                streak_good = false;
                break;
            }

            btwn += TimeDelta::days(1);
        }
        if !streak_good {
            break;
        }
        streak += 1;
    }
    streak
}

#[derive(Debug, Clone, Copy)]
pub struct Day {
    pub start: DateTime<Tz>,
    pub end: DateTime<Tz>,
}

impl Day {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        let start = start.with_timezone(&Oslo);
        let end = end.with_timezone(&Oslo);
        Self { start, end }
    }

    pub fn to_span(&self) -> Vec<Span<'_>> {
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
