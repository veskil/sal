use ratatui::{
    crossterm::event::Event,
    layout::{Constraint, Flex, Layout, Rect},
    widgets::Clear,
    Frame,
};
use tui_textarea::{CursorMove, Input, Key};

use crate::{models::Person, App};

pub fn handle_username_input(input: Event, app: &mut App) {
    match input.into() {
        Input {
            key: Key::Char('m'),
            ctrl: true,
            alt: false,
            shift: false,
        } => {
            // Ignore newline
        }
        Input {
            key: Key::Enter, ..
        } => {
            if let Some(user) = &mut app.current_user {
                let uid = user.id;
                let username = &app.textarea.lines()[0];
                user.set_username(username);
                app.current_user = Some(Person::load(uid));
                clear_popup(app);
            }
        }
        Input {
            key: Key::Esc,
            ctrl: _,
            alt: _,
            shift: _,
        } => {
            clear_popup(app);
        }
        input => {
            app.textarea.input(input);
        }
    };
}

fn clear_popup(app: &mut App) {
    app.textarea.move_cursor(CursorMove::End);
    app.textarea.delete_line_by_head();
    app.reading_username = false;
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

pub fn render_username_popup(frame: &mut Frame, app: &App, area: Rect) {
    let area = popup_area(area, 60, 10);
    frame.render_widget(&Clear, area);
    frame.render_widget(&app.textarea, area);
}
