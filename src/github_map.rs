use chrono::{Datelike, TimeDelta, Utc};
use chrono_tz::Europe::Oslo;
use itertools::Itertools;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style, Styled},
    text::Span,
    widgets::{Block, Widget},
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

const MS_IN_HOUR: u64 = 60 * 60 * 1000;
const MS_IN_2_HOURS: u64 = MS_IN_HOUR * 2;
const MS_IN_4_HOURS: u64 = MS_IN_HOUR * 4;
const MS_IN_8_HOURS: u64 = MS_IN_HOUR * 8;
const MS_IN_10_HOURS: u64 = MS_IN_HOUR * 10;
const MS_IN_12_HOURS: u64 = MS_IN_HOUR * 12;

fn ms_to_color(milliseconds: Option<u64>) -> Color {
    match milliseconds {
        None => Color::DarkGray,
        Some(milliseconds) => match milliseconds {
            0..MS_IN_HOUR => Color::LightBlue,
            0..MS_IN_2_HOURS => Color::Rgb(0, 240, 0),
            0..MS_IN_4_HOURS => Color::Rgb(0, 180, 0),
            0..MS_IN_8_HOURS => Color::Rgb(0, 120, 0),
            0..MS_IN_10_HOURS => Color::Rgb(0, 60, 0),
            0..MS_IN_12_HOURS => Color::Rgb(180, 0, 0),
            _ => Color::Rgb(60, 0, 0),
        },
    }
}

impl<'a> Widget for GithubMap<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = 8;
        let height = 4;

        let cols = (1..)
            .map(|i| area.x + area.width - width * i)
            .take_while_inclusive(|col| *col > width);

        let rows = (0..7).map(|i| area.y + height * i).rev();
        let squares = cols.cartesian_product(rows).map(|(col, row)| Rect {
            x: col,
            y: row,
            width,
            height,
        });

        let colors = self.values.iter().map(|val| ms_to_color(*val));

        let days_to_skip = 7
            - (Utc::now() - TimeDelta::hours(5))
                .with_timezone(&Oslo)
                .weekday()
                .num_days_from_sunday();

        for (color, square) in colors.zip(squares.skip(days_to_skip as usize)) {
            Block::bordered()
                .border_style(
                    Style::default()
                        .bg(Color::Black)
                        .fg(Color::Rgb(210, 210, 210)),
                )
                .border_type(ratatui::widgets::BorderType::QuadrantInside)
                .style(Style::default().bg(color))
                .render(square, buf);
        }
    }
}

pub fn github_map_instructions() -> Vec<Span<'static>> {
    let colors = [
        (None, " Ingen oppmøte "),
        (Some(0), " Mindre enn en time/bare ett bip "),
        (Some(MS_IN_HOUR), " En til to timer "),
        (Some(MS_IN_2_HOURS), " To til fire timer "),
        (Some(MS_IN_4_HOURS), " Fire til åtte timer "),
        (Some(MS_IN_8_HOURS), " Åtte til ti timer "),
        (Some(MS_IN_10_HOURS), " Ti til tolv timer "),
        (Some(MS_IN_12_HOURS), " Over tolv timer "),
    ]
    .map(|(time, text)| text.set_style(ms_to_color(time)));

    colors.to_vec()
}
