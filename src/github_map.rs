use itertools::Itertools;
use ratatui::{
    buffer::Buffer,
    crossterm::terminal::LeaveAlternateScreen,
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::{Color, Style},
    widgets::{self, Block, Paragraph, Widget},
};

pub struct GithubMap<'a> {
    values: &'a [Option<u64>],
}

impl<'a> GithubMap<'a> {
    pub fn new(day_fractions: &'a [Option<u64>]) -> Self {
        Self {
            values: day_fractions,
        }
    }
}

/// From a given parent rect, calculate the rect of the given index
fn idx_to_rect(idx: usize, area: Rect) -> Option<Rect> {
    let y = (idx % 4) as u16;
    let x = (idx / 4) as u16;

    let w = area.width / 10;
    let h = area.height / 4;

    let x = x * w;
    let y = y * h;

    let Some(y) = area.width.checked_sub(y) else {
        return None;
    };

    Some(Rect {
        x: area.x + x,
        y: area.y + y,
        width: w - 1,
        height: h - 1,
    })
}

const MS_IN_QUARTER: u64 = 15 * 60 * 1000;
const MS_IN_HOUR: u64 = 60 * 60 * 1000;
const MS_IN_2_HOURS: u64 = MS_IN_HOUR * 2;
const MS_IN_4_HOURS: u64 = MS_IN_HOUR * 4;
const MS_IN_8_HOURS: u64 = MS_IN_HOUR * 8;
const MS_IN_10_HOURS: u64 = MS_IN_HOUR * 10;
const MS_IN_12_HOURS: u64 = MS_IN_HOUR * 12;

fn ms_to_color(milliseconds: u64) -> Color {
    match milliseconds {
        0..MS_IN_HOUR => Color::LightBlue,
        0..MS_IN_2_HOURS => Color::Rgb(0, 240, 0),
        0..MS_IN_4_HOURS => Color::Rgb(0, 180, 0),
        0..MS_IN_8_HOURS => Color::Rgb(0, 120, 0),
        0..MS_IN_10_HOURS => Color::Rgb(0, 60, 0),
        0..MS_IN_12_HOURS => Color::Rgb(180, 0, 0),
        _ => Color::Rgb(60, 0, 0),
    }
}

impl<'a> Widget for GithubMap<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = 8;
        let height = 4;

        let cols = (1..)
            .map(|i| area.x + area.width - width * i)
            .take_while_inclusive(|col| *col > width);

        let rows = (0..4).map(|i| area.y + height * i).rev();
        let squares = cols.cartesian_product(rows).map(|(col, row)| Rect {
            x: col,
            y: row,
            width,
            height,
        });

        let colors = self.values.iter().map(|val| match val {
            None => Color::Rgb(180, 180, 180),
            Some(val) => ms_to_color(*val),
        });

        for (color, square) in colors.zip(squares) {
            Block::bordered()
                .border_style(Style::default().bg(Color::Black))
                .border_type(ratatui::widgets::BorderType::QuadrantInside)
                .style(Style::default().bg(color))
                .render(square, buf);
        }
    }
}
