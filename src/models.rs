use chrono::{DateTime, Datelike, NaiveDate, TimeDelta, Utc, Weekday};
use chrono_tz::{Europe::Oslo, Tz};
use itertools::Itertools;
use rusqlite::Connection;

pub fn get_db() -> Connection {
    Connection::open("sal.db").unwrap()
}

/// Tablename `logs`
#[allow(unused)]
#[derive(Debug)]
pub struct Log {
    timestamp: DateTime<Utc>,
    /// Date of beep-time minus five hours in Oslo time
    date: NaiveDate,
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
        let now = Utc::now();
        let date = (now.with_timezone(&Oslo) - TimeDelta::hours(5)).date_naive();
        conn.execute(
            "INSERT INTO logs (id, timestamp, date) VALUES (?1, ?2, ?3)",
            (&uid, &now, &date),
        )
        .unwrap();
    }
}

#[derive(Debug)]
pub struct Stats {
    pub streak: usize,
    pub longest_day: Day,
    pub today: Day,
    pub earliest_arrival: Day,
    pub latest_departure: Day,
    pub days_milliseconds: Vec<Option<u64>>,
}

impl Stats {
    fn load_for_user(uid: u64, uid2: Option<u64>) -> Self {
        let conn = get_db();

        let days = get_days(uid, uid2, conn);
        let streak = get_streak(&days);
        let today = days[0];
        let longest_day = get_longest_day(&days);
        let earliest_arrival = get_earliest(&days);
        let latest_departure = get_latest(&days);
        let fractions = get_milliseconds(&days);

        Self {
            streak,
            longest_day,
            today,
            earliest_arrival,
            latest_departure,
            days_milliseconds: fractions,
        }
    }
}

fn get_days(uid: u64, uid2: Option<u64>, conn: Connection) -> Vec<Day> {
    let query = "
    SELECT
        date,
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
        date DESC
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

fn get_earliest(days: &[Day]) -> Day {
    *days
        .into_iter()
        .min_by_key(|d| (d.start - TimeDelta::hours(5)).time())
        .unwrap()
}

fn get_latest(days: &[Day]) -> Day {
    *days
        .into_iter()
        .max_by_key(|d| (d.end - TimeDelta::hours(5)).time())
        .unwrap()
}

pub const MS_IN_A_DAY: u64 = 24 * 60 * 60 * 1000;

fn get_milliseconds(days: &[Day]) -> Vec<Option<u64>> {
    let last_day = days[0].date();
    let first_day = days[days.len() - 1].date();
    let num_days = (last_day - first_day).num_days();
    let mut milliseconds = Vec::with_capacity(num_days as usize);
    // From today, backwards
    for (day, prev_day) in days.iter().circular_tuple_windows() {
        milliseconds.push(Some(day.span().num_milliseconds() as u64));
        let mut moving_day = day.start - TimeDelta::days(1) - TimeDelta::hours(5);
        while moving_day.date_naive() > prev_day.date() {
            milliseconds.push(None);
            moving_day -= TimeDelta::days(1);
        }
    }

    milliseconds
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

    pub fn stats(&self) -> DayStats {
        let diff = self.end - self.start;
        let diff_formatted = match diff.num_hours() {
            1.. => format!(
                "{} timer og {} minutter",
                diff.num_hours(),
                diff.num_minutes() % 60
            ),
            ..=0 => format!(
                "{} minutter og {} sekunder",
                diff.num_minutes() % 60,
                diff.num_seconds() % 60
            ),
        };
        DayStats {
            date: self.start.format("%d/%m").to_string(),
            start: self.start.format("%H:%M").to_string(),
            end: self.end.format("%H:%M").to_string(),
            diff: diff_formatted,
        }
    }

    pub fn span(&self) -> TimeDelta {
        self.end - self.start
    }

    pub fn date(&self) -> NaiveDate {
        (self.start - TimeDelta::hours(5)).date_naive()
    }
}

pub struct DayStats {
    pub date: String,
    pub start: String,
    pub end: String,
    pub diff: String,
}
