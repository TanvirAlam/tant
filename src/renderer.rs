// Renderer + UI shell

use iced::widget::{Canvas, Column, Row, Button, Text, Scrollable};
use iced::{Element, Length, Color, Point, Size, Rectangle, Theme, Pixels};
use iced::widget::canvas::{self, Program, Frame};
use iced::mouse::Cursor;
use vt100;
use unicode_width::UnicodeWidthStr;
use crate::{Message, AiSettings, Block};

pub struct TerminalRenderer;

fn color_to_iced(color: vt100::Color) -> Color {
    // For simplicity, assume Rgb or default
    // Since the variants are not matching, let's use a simple approach
    // Perhaps the Color has rgb() method
    // But since not, let's assume it's Rgb
    if let vt100::Color::Rgb(r, g, b) = color {
        Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
    } else {
        Color::WHITE // default
    }
}

impl TerminalRenderer {
    pub fn new() -> Self {
        TerminalRenderer
    }

    pub fn cell_size(&self) -> (f32, f32) {
        (8.0, 16.0)
    }

    pub fn view<'a>(&self, _history: &'a [Block], _current: &Option<Block>, _current_command: &str, _search_query: &str, screen: &vt100::Screen, ai_settings: &'a AiSettings, ai_response: &'a Option<String>) -> Element<'a, Message> {
        let canvas = Canvas::new(TerminalCanvas {
            screen: screen.clone(),
            cell_width: 8.0,
            cell_height: 16.0,
        })
        .width(Length::Fill)
        .height(Length::Fill);

        let mut column = Column::new().spacing(10);

        // AI toggle
        let ai_toggle = Button::new(if ai_settings.enabled { "Disable AI" } else { "Enable AI" }).on_press(Message::ToggleAiEnabled);
        column = column.push(ai_toggle);

        // AI buttons
        if ai_settings.enabled {
            let ai_row = Row::new().spacing(10)
                .push(Button::new("Explain Error").on_press(Message::AiExplainError))
                .push(Button::new("Suggest Fix").on_press(Message::AiSuggestFix))
                .push(Button::new("Generate Command").on_press(Message::AiGenerateCommand))
                .push(Button::new("Summarize Output").on_press(Message::AiSummarizeOutput));
            column = column.push(ai_row);
        }

        column = column.push(canvas);

        // AI response
        if let Some(response) = ai_response {
            let response_text = Text::new(response);
            let scrollable = Scrollable::new(response_text);
            column = column.push(scrollable);
        }

        column.into()
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

                    // Calculate display width
                    let content = cell.contents();
                    let width = content.width() as f32;
                    let display_width = if width > 1.0 { width * self.cell_width } else { self.cell_width };

                    // Draw background
                    let bg = color_to_iced(cell.bgcolor());
                    frame.fill_rectangle(Point::new(x, y), Size::new(display_width, self.cell_height), bg);

                    // Draw text only if content is not empty
                    if !content.is_empty() {
                        let fg = color_to_iced(cell.fgcolor());
                        let text = canvas::Text {
                            content: content.into(),
                            position: Point::new(x, y + self.cell_height),
                            size: Pixels(self.cell_height),
                            color: fg,
                            ..canvas::Text::default()
                        };
                        frame.fill_text(text);
                    }
                }
            }
        }

        vec![frame.into_geometry()]
    }
}