use std::rc::Rc;

use chrono::{DateTime, Datelike, NaiveDate, TimeDelta, Utc, Weekday};
use chrono_tz::{Europe::Oslo, Tz};
use itertools::Itertools;
use rusqlite::{types::Value, vtab::array, Connection};

pub fn get_db() -> Connection {
    let db = Connection::open("sal.db").unwrap();
    array::load_module(&db).unwrap();
    db
}

/// Tablename `logs`
#[allow(unused)]
#[derive(Debug)]
pub struct Log {
    timestamp: DateTime<Utc>,
    /// Date of beep-time minus five hours in Oslo time
    date: NaiveDate,
    id: u32,
}

/// Tablename `people`
#[allow(unused)]
#[derive(Debug)]
pub struct Person {
    pub id: u32,
    pub ids: Vec<u32>,
    pub username: String,
    pub stats: Stats,
}

impl Person {
    pub fn load(uid: u32) -> Self {
        let conn = get_db();
        let username_res =
            conn.query_row("SELECT username FROM people WHERE id=($1)", (uid,), |row| {
                let un = row.get(0).unwrap();
                Ok(un)
            });
        // .unwrap_or_else(|uid| uid.to_string());
        let (username, ids) = if let Ok(username) = username_res {
            let mut ids_stmt = conn
                .prepare("SELECT id FROM people WHERE USERNAME=($1)")
                .unwrap();

            let ids = ids_stmt
                .query_map([&username], |row| {
                    let id: u32 = row.get(0).unwrap();

                    Ok(id)
                })
                .unwrap();
            let ids: Result<Vec<_>, _> = ids.collect();
            let ids = ids.unwrap();
            assert!(
                ids.len() != 0,
                "Failed to load IDs of user. Should never happen."
            );
            (username, ids)
        } else {
            (uid.to_string(), vec![uid])
        };

        let stats = Stats::load_for_user(&ids);

        Self {
            id: uid,
            ids,
            username,
            stats,
        }
    }

    pub fn register(uid: u32) {
        let conn = get_db();
        let now = Utc::now();
        let date = (now.with_timezone(&Oslo) - TimeDelta::hours(5)).date_naive();
        conn.execute(
            "INSERT INTO logs (id, timestamp, date) VALUES (?1, ?2, ?3)",
            (&uid, &now, &date),
        )
        .unwrap();
    }

    pub fn set_username(&self, username: &str) {
        let conn = get_db();
        conn.execute(
            "INSERT INTO people (id, username) VALUES (?1, ?2) 
                    ON CONFLICT (id) DO UPDATE SET username=excluded.username",
            (&self.id, username),
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
    pub days: Vec<DayOrDate>,
    pub days_milliseconds: Vec<Option<u64>>,
    pub last_week_count: usize,
    pub last_month_count: usize,
}

impl Stats {
    fn load_for_user(ids: &[u32]) -> Self {
        let conn = get_db();

        let days = get_days(ids, conn);
        let day_or_dates = days.as_slice().iter_option();
        let streak = get_streak(&day_or_dates);
        let today = days[0];
        let longest_day = get_longest_day(&days);
        let earliest_arrival = get_earliest(&days);
        let latest_departure = get_latest(&days);
        let days_milliseconds = get_milliseconds(&day_or_dates);
        let last_week_count = get_last_n(7, &day_or_dates);
        let last_month_count = get_last_n(30, &day_or_dates);

        Self {
            streak,
            longest_day,
            today,
            earliest_arrival,
            latest_departure,
            days: day_or_dates,
            days_milliseconds,
            last_week_count,
            last_month_count,
        }
    }
}

fn get_days(ids: &[u32], conn: Connection) -> Vec<Day> {
    assert!(ids.len() != 0, "Cannot get the days of nobody");
    let placeholders = std::iter::repeat("?")
        .take(ids.len())
        .collect::<Vec<_>>()
        .join(",");

    let query = "
    SELECT
        date,
        MIN(timestamp) AS first_timestamp,
        MAX(timestamp) AS last_timestamp,
        JULIANDAY(MAX(timestamp)) - JULIANDAY(MIN(timestamp)) AS difference_in_days
    FROM
        logs
    WHERE
        id IN rarray(?)
    GROUP BY
        date
    ORDER BY
        date DESC
    ";

    let mut stmt = conn.prepare(query).unwrap();
    let ids = ids.iter().copied().map(Value::from).collect_vec();
    let ids = Rc::new(ids);
    let days = stmt
        .query_map([ids], |row| {
            let date: NaiveDate = row.get(0).unwrap();
            let start: DateTime<Utc> = row.get(1).unwrap();
            let end: DateTime<Utc> = row.get(2).unwrap();

            Ok(Day::new(date, start, end))
        })
        .unwrap();
    let days: Result<Vec<_>, _> = days.collect();

    let days = days.unwrap();

    assert!(
        days.len() != 0,
        "Since this only runs after inserting a day, days should never be empty"
    );
    days
}

fn get_longest_day(days: &[Day]) -> Day {
    *days.iter().max_by_key(|day| day.span()).unwrap()
}

fn get_streak(days: &[DayOrDate]) -> usize {
    days.iter()
        .take_while(|day| match day {
            DayOrDate::Unregistered(date) => {
                date.weekday() == Weekday::Sun || date.weekday() == Weekday::Sat
            }
            DayOrDate::Registered(_) => true,
        })
        .filter(|day| day.is_registered())
        .count()
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

fn get_milliseconds(days: &[DayOrDate]) -> Vec<Option<u64>> {
    days.iter()
        .map(|day| match day {
            DayOrDate::Unregistered(_) => None,
            DayOrDate::Registered(day) => Some(day.span().num_milliseconds() as u64),
        })
        .collect()
}

fn get_last_n(n: usize, days: &[DayOrDate]) -> usize {
    days.iter()
        .take(n)
        .filter(|day| day.is_registered())
        .count()
}

#[derive(Debug, Clone, Copy)]
pub struct Day {
    pub date: NaiveDate,
    pub start: DateTime<Tz>,
    pub end: DateTime<Tz>,
}

impl Day {
    pub fn new(date: NaiveDate, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        let start = start.with_timezone(&Oslo);
        let end = end.with_timezone(&Oslo);
        Self { date, start, end }
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
}

pub struct DayStats {
    pub date: String,
    pub start: String,
    pub end: String,
    pub diff: String,
}

#[derive(Debug, Clone, Copy)]
pub enum DayOrDate {
    Registered(Day),
    Unregistered(NaiveDate),
}

impl DayOrDate {
    fn date(&self) -> NaiveDate {
        match self {
            &DayOrDate::Registered(day) => day.date,
            &DayOrDate::Unregistered(date) => date,
        }
    }

    fn is_registered(&self) -> bool {
        match self {
            DayOrDate::Registered(_) => true,
            DayOrDate::Unregistered(_) => false,
        }
    }
}

pub trait DayVec {
    fn iter_option(&self) -> Vec<DayOrDate>;
}

impl DayVec for &[Day] {
    fn iter_option(&self) -> Vec<DayOrDate> {
        let n_days = (self[0].date - self.last().unwrap().date).num_days() + 1;
        let mut output = Vec::with_capacity(n_days as usize);
        for (day, prev_day) in self.iter().tuple_windows() {
            output.push(DayOrDate::Registered(*day));
            let day = day.date;
            let prev_day = prev_day.date;
            if day == prev_day {
                continue;
            }

            let mut btwn = day - TimeDelta::days(1);
            while btwn != prev_day {
                output.push(DayOrDate::Unregistered(btwn));

                btwn -= TimeDelta::days(1);
            }
        }
        output
    }
}
