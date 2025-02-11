mod custom_sparkline;
mod github_map;
mod migrate;
mod models;

use std::io;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use github_map::GithubMap;
use migrate::{dump, migrate};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::widgets::canvas::Canvas;
use ratatui::{
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Sparkline},
    DefaultTerminal, Frame,
};

use models::{Person, MS_IN_A_DAY};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Migrate data from log files into sqlite
    Migrate,

    /// Dump data from db into log file format
    Dump,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    if let Some(command) = &cli.command {
        match command {
            Commands::Migrate => return migrate(),
            Commands::Dump => return dump(),
        }
    }

    let mut terminal = ratatui::init();
    let app_result = App::new().run(&mut terminal);
    ratatui::restore();
    println!("Salstatistikk avsluttet eller crashet. For å starte på nytt, klikk pil opp og enter eller skriv `cargo run`");
    app_result
}

#[derive(Debug)]
pub struct App {
    exit: bool,
    buffer: String,
    last_input: Instant,
    current_user: Option<Person>,
}

const TIMEOUT: Duration = Duration::from_millis(20);

impl App {
    fn new() -> Self {
        Self {
            exit: false,
            buffer: String::with_capacity(12),
            last_input: Instant::now(),
            current_user: None,
        }
    }

    fn on_tick(&mut self) {
        // Needed in case the card input is spread on two ticks
        // Which is quite common when the tick rate is only 200ms
        // Higher tick rate makes the program feel slow
        if self.last_input.elapsed() > TIMEOUT {
            self.process_buffer();
            self.buffer.clear();
            self.last_input = Instant::now();
        }
    }

    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        // 5 fps
        let tick_rate = Duration::from_millis(200);

        let mut last_tick = Instant::now();
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                self.handle_events()?;
            }
            if last_tick.elapsed() >= tick_rate {
                self.on_tick();
                last_tick = Instant::now();
            }
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let chunks = Layout::vertical([
            Constraint::Length(9),
            Constraint::Min(8),
            Constraint::Length(16),
        ])
        .split(frame.area());
        render_welcome_box(frame, self, chunks[0]);
        render_sparkline(frame, self, chunks[1]);
        render_github_stats(frame, self, chunks[2]);
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char(c) => {
                self.buffer.push(c);
                self.last_input = Instant::now();
            }
            KeyCode::Left => self.decrement_counter(),
            KeyCode::Right => self.increment_counter(),
            KeyCode::Esc => self.exit = true,
            _ => {}
        }
    }

    fn process_buffer(&mut self) {
        // if self.buffer.len() > 0 {
        //     println!("{}", self.buffer);
        // }
        if self.buffer.len() == 10 {
            let Ok(uid): Result<u64, _> = self.buffer.parse() else {
                return;
            };
            self.beep_user(uid);
        }
        if self.buffer.len() == 1 {
            let c = self.buffer.chars().next().unwrap();
            match c.to_ascii_lowercase() {
                'q' => self.exit = true,
                'm' => self.beep_user(394769250),
                'n' => self.beep_user(331142554),
                'b' => self.current_user = None,
                _ => (),
            }
        }
    }

    fn beep_user(&mut self, uid: u64) {
        Person::register(uid);

        self.current_user = Some(Person::load(uid));
    }

    fn increment_counter(&mut self) {}

    fn decrement_counter(&mut self) {}
}

fn render_welcome_box(frame: &mut Frame, app: &App, area: Rect) {
    let title = Line::from(" Salstatistikk ".bold());
    let instructions = Line::from(vec![
        " Decrement ".into(),
        "<Left>".blue().bold(),
        " Increment ".into(),
        "<Right>".blue().bold(),
        " Quit ".into(),
        "<Q> ".blue().bold(),
    ]);
    let block = Block::bordered()
        .title(title.centered())
        .title_bottom(instructions.centered())
        .border_set(border::THICK);

    let text = match &app.current_user {
        None => Text::from(vec![Line::from(vec!["Utlogget".yellow()])]),
        Some(user) => {
            let longest = user.stats.longest_day.stats();
            let today = user.stats.today.stats();
            let earliest_arrival = user.stats.earliest_arrival.stats();
            let latest_departure = user.stats.latest_departure.stats();
            Text::from(vec![
                Line::from(vec![
                    "Velkommen ".into(),
                    user.username.to_string().yellow(),
                ]),
                Line::from(vec!["🔥".repeat(user.stats.streak).into()]),
                Line::from(vec![
                    "I dag har du vært her fra ".into(),
                    today.start.yellow(),
                    " som blir ".into(),
                    today.diff.green(),
                ]),
                Line::from(vec![
                    "Lengste dag: ".into(),
                    longest.date.yellow(),
                    ". Fra ".into(),
                    longest.start.yellow(),
                    " til: ".into(),
                    longest.end.yellow(),
                    ". Det er hele ".into(),
                    longest.diff.green(),
                ]),
                Line::from(vec![
                    "Tidligste ankomst: ".into(),
                    earliest_arrival.start.yellow(),
                    " den ".into(),
                    earliest_arrival.date.yellow(),
                ]),
                Line::from(vec![
                    "Seneste avreise: ".into(),
                    latest_departure.end.yellow(),
                    " den ".into(),
                    latest_departure.date.yellow(),
                ]),
            ])
        }
    };

    let paragraph = Paragraph::new(text).centered().block(block);
    frame.render_widget(paragraph, area);
}

fn render_sparkline(frame: &mut Frame, app: &App, area: Rect) {
    let data = match &app.current_user {
        None => &vec![],
        Some(user) => &user.stats.days_milliseconds,
    };

    let maxval = data
        .iter()
        .chain(&[Some(MS_IN_A_DAY / 2)])
        .max()
        .copied()
        .flatten()
        .unwrap();

    let sparkline = Sparkline::default()
        .block(
            Block::bordered().title(format!("Timer per dag - max {}", maxval / (60 * 60 * 1000))),
        )
        .direction(ratatui::widgets::RenderDirection::RightToLeft)
        .data(data)
        .max(maxval);

    frame.render_widget(sparkline, area);
}

fn render_github_stats(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(user) = &app.current_user {
        let gh_map = GithubMap::new(&user.stats.days_milliseconds);
        frame.render_widget(gh_map, area);
    }
}
