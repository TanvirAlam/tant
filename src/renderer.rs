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

    pub fn view<'a>(&self, _history: &'a [Block], _current: &Option<Block>, current_command: &'a str, _search_query: &str, screen: &vt100::Screen, ai_settings: &'a AiSettings, _ai_response: &'a Option<String>, scroll_offset: usize, selection_start: Option<(usize, usize)>, selection_end: Option<(usize, usize)>) -> Element<'a, Message> {
        // Terminal canvas - absolute full width and height
        let canvas = Canvas::new(TerminalCanvas {
            screen: screen.clone(),
            cell_width: 8.0,
            cell_height: 16.0,
            scroll_offset,
            selection_start,
            selection_end,
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

        // Copy button if selection active
        if selection_start.is_some() {
            top_row = top_row.push(Button::new("Copy").on_press(Message::CopySelected));
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
    pub scroll_offset: usize,
    pub selection_start: Option<(usize, usize)>,
    pub selection_end: Option<(usize, usize)>,
}

impl Program<Message> for TerminalCanvas {
    type State = ();

    fn draw(&self, _state: &Self::State, renderer: &iced::Renderer, _theme: &Theme, bounds: Rectangle, _cursor: Cursor) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        // Fill with default background
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), default_bg_color());

        let visible_rows = (bounds.height / self.cell_height) as usize;
        let contents = self.screen.contents();
        let lines: Vec<&str> = contents.lines().collect();
        let total_lines = lines.len();
        let start_line = total_lines.saturating_sub(visible_rows + self.scroll_offset);

        eprintln!("Canvas bounds: {:?}, Total lines: {}, Start line: {}, Scroll offset: {}", bounds, total_lines, start_line, self.scroll_offset);

        for y in 0..visible_rows {
            if let Some(line) = lines.get(start_line + y) {
                let line_idx = start_line + y;
                let (start_col, end_col) = if let (Some(s), Some(e)) = (self.selection_start, self.selection_end) {
                    let (min_l, min_c) = s.min(e);
                    let (max_l, max_c) = s.max(e);
                    if line_idx >= min_l && line_idx <= max_l {
                        let sc = if line_idx == min_l { min_c } else { 0 };
                        let ec = if line_idx == max_l { max_c } else { line.len() };
                        (sc, ec)
                    } else {
                        (0, 0)
                    }
                } else {
                    (0, 0)
                };

                let y_pos = y as f32 * self.cell_height;
                if start_col < end_col && start_col < line.len() {
                    let end_col = end_col.min(line.len());
                    let before = &line[0..start_col];
                    let selected = &line[start_col..end_col];
                    let after = &line[end_col..];

                    let mut x_pos = 0.0;

                    if !before.is_empty() {
                        frame.fill_text(canvas::Text {
                            content: before.to_string(),
                            position: Point::new(x_pos, y_pos),
                            size: Pixels(self.cell_height),
                            color: Color::from_rgb(0.9, 0.9, 0.9),
                            font: Font::MONOSPACE,
                            ..canvas::Text::default()
                        });
                        x_pos += before.len() as f32 * self.cell_width;
                    }

                    if !selected.is_empty() {
                        frame.fill_rectangle(Point::new(x_pos, y_pos), Size::new(selected.len() as f32 * self.cell_width, self.cell_height), Color::from_rgb(0.5, 0.5, 1.0));
                        frame.fill_text(canvas::Text {
                            content: selected.to_string(),
                            position: Point::new(x_pos, y_pos),
                            size: Pixels(self.cell_height),
                            color: Color::from_rgb(0.0, 0.0, 0.0),
                            font: Font::MONOSPACE,
                            ..canvas::Text::default()
                        });
                        x_pos += selected.len() as f32 * self.cell_width;
                    }

                    if !after.is_empty() {
                        frame.fill_text(canvas::Text {
                            content: after.to_string(),
                            position: Point::new(x_pos, y_pos),
                            size: Pixels(self.cell_height),
                            color: Color::from_rgb(0.9, 0.9, 0.9),
                            font: Font::MONOSPACE,
                            ..canvas::Text::default()
                        });
                    }
                } else {
                    frame.fill_text(canvas::Text {
                        content: line.to_string(),
                        position: Point::new(0.0, y_pos),
                        size: Pixels(self.cell_height),
                        color: Color::from_rgb(0.9, 0.9, 0.9),
                        font: Font::MONOSPACE,
                        ..canvas::Text::default()
                    });
                }
            }
        }

        vec![frame.into_geometry()]
    }
}