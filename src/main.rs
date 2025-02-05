mod migrate;
mod models;

use std::io;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use migrate::{dump, migrate};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
    DefaultTerminal, Frame,
};

use models::Person;

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
        frame.render_widget(self, frame.area());
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
                'n' => self.beep_user(394769250),
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

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
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

        let text = match &self.current_user {
            None => Text::from(vec![Line::from(vec!["Utlogget".yellow()])]),
            Some(user) => {
                let longest_day = user.stats.longest_day.to_span();
                Text::from(vec![
                    Line::from(vec![
                        "Velkommen ".into(),
                        user.username.to_string().yellow(),
                    ]),
                    Line::from(vec!["🔥".repeat(user.stats.streak).into()]),
                    Line::from(longest_day),
                ])
            }
        };

        Paragraph::new(text)
            .centered()
            .block(block)
            .render(area, buf);
    }
}
