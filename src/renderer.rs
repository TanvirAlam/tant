// Renderer + UI shell

use iced::widget::Canvas;
use iced::{Element, Length, Color, Point, Size, Rectangle, Theme};
use iced::widget::canvas::{self, Program, Frame, Text};
use iced::mouse::Cursor;
use vt100;

use crate::{Message, Block};

pub struct TerminalRenderer;

impl TerminalRenderer {
    pub fn new() -> Self {
        TerminalRenderer
    }

    pub fn cell_size(&self) -> (f32, f32) {
        (8.0, 16.0)
    }

    pub fn view<'a>(&self, history: &'a [Block], current: &Option<Block>, current_command: &str, search_query: &str, screen: &vt100::Screen) -> Element<'a, Message> {
        let canvas = Canvas::new(TerminalCanvas {
            screen: screen.clone(),
            cell_width: 8.0,
            cell_height: 16.0,
        })
        .width(Length::Fill)
        .height(Length::Fill);

        canvas.into()
    }
}

pub struct TerminalCanvas {
    pub screen: vt100::Screen,
    pub cell_width: f32,
    pub cell_height: f32,
}

impl Program<Message> for TerminalCanvas {
    type State = ();

    fn draw(&self, _state: &Self::State, renderer: &iced::Renderer, _theme: &Theme, bounds: Rectangle, _cursor: Cursor) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        let size = self.screen.size();
        let rows = size.1 as usize;
        let cols = size.0 as usize;

        for row in 0..rows {
            for col in 0..cols {
                if let Some(cell) = self.screen.cell(row as u16, col as u16) {
                    let x = col as f32 * self.cell_width;
                    let y = row as f32 * self.cell_height;

                    // Draw background
                    let bg_color = cell.bgcolor();
                    let bg = Color::from_rgb(bg_color.r() as f32 / 255.0, bg_color.g() as f32 / 255.0, bg_color.b() as f32 / 255.0);
                    frame.fill_rectangle(Point::new(x, y), Size::new(self.cell_width, self.cell_height), bg);

                    // Draw text
                    let fg_color = cell.fgcolor();
                    let fg = Color::from_rgb(fg_color.r() as f32 / 255.0, fg_color.g() as f32 / 255.0, fg_color.b() as f32 / 255.0);
                    frame.fill_text(Text {
                        content: cell.contents(),
                        position: Point::new(x, y + self.cell_height),
                        size: self.cell_height,
                        color: fg,
                        ..Text::default()
                    });
                }
            }
        }

        vec![frame.into_geometry()]
    }
}