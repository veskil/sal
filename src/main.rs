mod github_map;
mod migrate;
mod models;
mod username_popup;

use std::io;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use github_map::{github_map_instructions, GithubMap};
use itertools::Itertools;
use migrate::{dump, migrate};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::Style;
use ratatui::text::ToSpan;
use ratatui::widgets::Padding;
use ratatui::{
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph},
    DefaultTerminal, Frame,
};

use models::Person;
use tui_textarea::TextArea;
use username_popup::{handle_username_input, render_username_popup};

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
pub struct App<'a> {
    exit: bool,
    buffer: String,
    last_input: Instant,
    current_user: Option<Person>,
    textarea: TextArea<'a>,
    reading_username: bool,
}

const TIMEOUT: Duration = Duration::from_millis(20);

impl<'a> App<'a> {
    fn new() -> Self {
        let mut textarea = TextArea::default();
        textarea.set_style(Style::default().white().on_blue());
        textarea.set_block(
            Block::bordered()
                .white()
                .on_blue()
                .title("Oppdater brukernavn")
                .title_bottom("Avbryt <Esc> Bekreft <Enter>"),
        );

        Self {
            exit: false,
            buffer: String::with_capacity(12),
            last_input: Instant::now(),
            current_user: None,
            textarea,
            reading_username: false,
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
        let chunks = Layout::vertical([Constraint::Length(14), Constraint::Min(2 + 7 * 4)])
            .split(frame.area());
        render_welcome_box(frame, self, chunks[0]);
        render_github_stats(frame, self, chunks[1]);

        if self.reading_username {
            render_username_popup(frame, self, frame.area());
        }
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            // If the popup is open, it is responsible for handling inputs
            input if self.reading_username => handle_username_input(input, self),
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
            KeyCode::Left => self.decrement_counter(),
            KeyCode::Right => self.increment_counter(),
            KeyCode::Esc => self.exit = true,
            KeyCode::Char(c) if c.is_numeric() => {
                self.buffer.push(c);
                self.last_input = Instant::now();
            }
            KeyCode::Char(c) => match c.to_ascii_lowercase() {
                'b' => self.current_user = None,
                'u' => {
                    if let Some(_) = self.current_user {
                        self.reading_username = true
                    }
                }
                _ => (),
            },
            _ => {}
        }
    }

    fn process_buffer(&mut self) {
        // if self.buffer.len() > 0 {
        //     println!("{}", self.buffer);
        // }
        if self.buffer.len() == 10 {
            let Ok(uid): Result<u32, _> = self.buffer.parse() else {
                return;
            };
            self.beep_user(uid);
        }
    }

    fn beep_user(&mut self, uid: u32) {
        Person::register(uid);

        self.current_user = Some(Person::load(uid));
    }

    fn increment_counter(&mut self) {}

    fn decrement_counter(&mut self) {}
}

fn render_welcome_box(frame: &mut Frame, app: &App, area: Rect) {
    let title = Line::from(" Salstatistikk ".bold());
    let instructions = Line::from(vec![
        " Logg inn ".into(),
        "<Tæpp kortet>".blue().bold(),
        " Logg ut ".into(),
        "<B>".blue().bold(),
        " Endre brukernavn ".into(),
        "<U>".blue().bold(),
        " Lukk appen ".into(),
        "<Esc> ".blue().bold(),
    ]);
    let block = Block::bordered()
        .title(title.centered())
        .title_bottom(instructions.centered())
        .border_set(border::THICK);

    let text = match &app.current_user {
        None => {
            let _ = 1;

            Text::from(vec![
                Line::from("Instruksjoner: ".blue()),
                Line::from(""),
                Line::from("Tæpp NTNU-kortet ditt på kortleseren for å registrere oppmøte."),
                Line::from("Det er kun første og siste tæpp for dagen* som har noe å si for statistikken, de tolkes som ankomst og avreise"),
                Line::from(""),
                Line::from("Noen kort inneholder to nummer. Dersom du registrerer begge på samme brukernavn blir de ansett som samme bruker."),
                Line::from(""),
                Line::from("*Dagen varer fra 05:00 til 04:59.".italic()),
            ].into_iter().map(|line| line.left_aligned()).collect_vec())
        }
        Some(user) => {
            let longest = user.stats.longest_day.stats();
            let today = user.stats.today.stats();
            let earliest_arrival = user.stats.earliest_arrival.stats();
            let latest_departure = user.stats.latest_departure.stats();
            Text::from(
                vec![
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
                    Line::from(vec![
                        "Antall møtte siste syv dager: ".into(),
                        user.stats.last_week_count.to_span().yellow(),
                    ]),
                    Line::from(vec![
                        "Antall møtte siste 30 dager: ".into(),
                        user.stats.last_month_count.to_span().yellow(),
                    ]),
                ]
                .into_iter()
                .map(|line| line.left_aligned())
                .collect_vec(),
            )
        }
    };

    let paragraph = Paragraph::new(text)
        .centered()
        .block(block.padding(Padding::symmetric(5, 1)));
    frame.render_widget(paragraph, area);
}

fn render_github_stats(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(user) = &app.current_user {
        let title = Line::centered(" Oppmøtehistorikk ".into()).blue();
        let instrs = Line::from(github_map_instructions()).centered();
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instrs.bold())
            .border_set(border::THICK)
            .padding(Padding::symmetric(5, 1));

        frame.render_widget(&block, area);
        let inner = block.inner(area);
        let gh_map = GithubMap::new(&user.stats.days_milliseconds);
        frame.render_widget(gh_map, inner);
    }
}
