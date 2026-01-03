// Renderer + UI shell

use iced::canvas::{self, Canvas, Frame, Text};
use iced::{Color, Element, Font, Point, Size};
use vt100::Screen;

use crate::Message;

pub struct TerminalRenderer {
    cell_width: f32,
    cell_height: f32,
}

impl TerminalRenderer {
    pub fn new() -> Self {
        // Hardcode cell size for monospace font
        TerminalRenderer {
            cell_width: 8.0,
            cell_height: 16.0,
        }
    }

    pub fn cell_size(&self) -> (f32, f32) {
        (self.cell_width, self.cell_height)
    }

    pub fn view(&self, screen: &Screen) -> Element<Message> {
        Canvas::new(TerminalCanvas {
            screen: screen.clone(),
            cell_width: self.cell_width,
            cell_height: self.cell_height,
        })
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
    }
}

struct TerminalCanvas {
    screen: Screen,
    cell_width: f32,
    cell_height: f32,
}

impl canvas::Program<Message> for TerminalCanvas {
    fn draw(&self, bounds: iced::Rectangle, _cursor: canvas::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(bounds.size());

        // Draw background
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::BLACK);

        // Draw cells
        for row in 0..self.screen.size().1 as usize {
            for col in 0..self.screen.size().0 as usize {
                if let Some(cell) = self.screen.cell(row as u16, col as u16) {
                    let x = col as f32 * self.cell_width;
                    let y = row as f32 * self.cell_height;
                    let size = Size::new(self.cell_width, self.cell_height);

                    // Background
                    let bg_color = color_from_vt100(cell.bg());
                    frame.fill_rectangle(Point::new(x, y), size, bg_color);

                    // Text
                    let fg_color = color_from_vt100(cell.fg());
                    let text = Text {
                        content: cell.contents(),
                        position: Point::new(x, y + self.cell_height * 0.8), // Adjust baseline
                        color: fg_color,
                        size: 12.0,
                        font: Font::MONOSPACE,
                        ..Default::default()
                    };
                    frame.fill_text(text);
                }
            }
        }

        vec![frame.into_geometry()]
    }
}

fn color_from_vt100(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::WHITE,
        vt100::Color::Idx(i) => match i {
            0 => Color::BLACK,
            1 => Color::RED,
            2 => Color::GREEN,
            3 => Color::YELLOW,
            4 => Color::BLUE,
            5 => Color::MAGENTA,
            6 => Color::CYAN,
            7 => Color::WHITE,
            // Add more if needed
            _ => Color::WHITE,
        },
        vt100::Color::Rgb(r, g, b) => Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0),
    }
}