// Renderer + UI shell

use iced::widget::{Canvas, Column, Row, Button, TextInput};
use iced::{Element, Length, Color, Point, Size, Rectangle, Theme, Pixels, Font};
use iced::widget::canvas::{self, Program, Frame};
use iced::mouse::Cursor;
use vt100;
use unicode_width::UnicodeWidthStr;
use crate::{Message, AiSettings, Block};

pub struct TerminalRenderer;

fn color_to_iced(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Rgb(r, g, b) => Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0),
        vt100::Color::Idx(idx) => {
            // Standard ANSI colors
            match idx {
                0 => Color::from_rgb(0.0, 0.0, 0.0),       // Black
                1 => Color::from_rgb(0.8, 0.0, 0.0),       // Red
                2 => Color::from_rgb(0.0, 0.8, 0.0),       // Green
                3 => Color::from_rgb(0.8, 0.8, 0.0),       // Yellow
                4 => Color::from_rgb(0.0, 0.0, 0.8),       // Blue
                5 => Color::from_rgb(0.8, 0.0, 0.8),       // Magenta
                6 => Color::from_rgb(0.0, 0.8, 0.8),       // Cyan
                7 => Color::from_rgb(0.9, 0.9, 0.9),       // White
                8 => Color::from_rgb(0.5, 0.5, 0.5),       // Bright Black
                9 => Color::from_rgb(1.0, 0.0, 0.0),       // Bright Red
                10 => Color::from_rgb(0.0, 1.0, 0.0),      // Bright Green
                11 => Color::from_rgb(1.0, 1.0, 0.0),      // Bright Yellow
                12 => Color::from_rgb(0.0, 0.0, 1.0),      // Bright Blue
                13 => Color::from_rgb(1.0, 0.0, 1.0),      // Bright Magenta
                14 => Color::from_rgb(0.0, 1.0, 1.0),      // Bright Cyan
                15 => Color::from_rgb(1.0, 1.0, 1.0),      // Bright White
                _ => Color::from_rgb(0.9, 0.9, 0.9),       // Default to white
            }
        }
        vt100::Color::Default => Color::from_rgb(0.9, 0.9, 0.9), // Default foreground
    }
}

fn bgcolor_to_iced(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => default_bg_color(), // Use ash gray for default background
        _ => color_to_iced(color), // Use regular color mapping for explicit colors
    }
}

fn default_bg_color() -> Color {
    Color::from_rgb(0.15, 0.15, 0.15) // Dark ash gray background
}

impl TerminalRenderer {
    pub fn new() -> Self {
        TerminalRenderer
    }

    pub fn cell_size(&self) -> (f32, f32) {
        (8.0, 16.0)
    }

    pub fn view<'a>(&self, _history: &'a [Block], _current: &Option<Block>, current_command: &'a str, _search_query: &str, screen: &vt100::Screen, ai_settings: &'a AiSettings, _ai_response: &'a Option<String>) -> Element<'a, Message> {
        // Terminal canvas - absolute full width and height
        let canvas = Canvas::new(TerminalCanvas {
            screen: screen.clone(),
            cell_width: 8.0,
            cell_height: 16.0,
        })
        .width(Length::Fill)
        .height(Length::Fill);

        // Simple column with minimal spacing
        let mut main_column = Column::new()
            .spacing(0)
            .padding(0)
            .width(Length::Fill)
            .height(Length::Fill);

        // Top bar with AI controls - minimal
        let mut top_row = Row::new().spacing(3).padding([2, 5]);
        let ai_toggle = Button::new(if ai_settings.enabled { "Disable AI" } else { "Enable AI" })
            .on_press(Message::ToggleAiEnabled);
        top_row = top_row.push(ai_toggle);

        // AI buttons - only show if enabled
        if ai_settings.enabled {
            top_row = top_row
                .push(Button::new("Explain").on_press(Message::AiExplainError))
                .push(Button::new("Fix").on_press(Message::AiSuggestFix))
                .push(Button::new("Cmd").on_press(Message::AiGenerateCommand))
                .push(Button::new("Sum").on_press(Message::AiSummarizeOutput));
        }
        
        main_column = main_column.push(top_row);
        
        // Text input for commands
        let text_input = TextInput::new("Type command...", current_command)
            .on_input(Message::TerminalInput)
            .on_submit(Message::TerminalSubmit)
            .padding(3)
            .width(Length::Fill);
        
        main_column = main_column.push(text_input);

        // Terminal canvas - this should take ALL remaining vertical space
        main_column = main_column.push(canvas);

        main_column.into()
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

        // Fill with default background
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), default_bg_color());

        let size = self.screen.size();
        let rows = size.1 as usize;
        let cols = size.0 as usize;
        
        eprintln!("Canvas bounds: {:?}, Screen size: {}x{}", bounds, cols, rows);

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
                    let bg = bgcolor_to_iced(cell.bgcolor());
                    frame.fill_rectangle(Point::new(x, y), Size::new(display_width, self.cell_height), bg);

                    // Draw text only if content is not empty
                    if !content.is_empty() && content != " " {
                        let fg = color_to_iced(cell.fgcolor());
                        let text = canvas::Text {
                            content: content.into(),
                            position: Point::new(x, y),
                            size: Pixels(self.cell_height),
                            color: fg,
                            font: Font::MONOSPACE,
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