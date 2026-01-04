// Renderer + UI shell

use iced::widget::{Canvas, Column, Row, Text, Scrollable, Container};
use iced::widget::button::Button;
use iced::widget::text_input::TextInput;
use iced::{Element, Length, Color, Point, Size, Rectangle, Theme, Pixels, Font, Alignment};
use iced::widget::canvas::{self, Program, Frame};
use iced::mouse::Cursor;
use vt100;
use chrono::Utc;
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

    pub fn view<'a>(&self, history: &'a [Block], current: &'a Option<Block>, current_command: &'a str, _search_query: &str, screen: &vt100::Screen, alt_screen_active: bool, _ai_settings: &'a AiSettings, _ai_response: &'a Option<String>, _scroll_offset: usize, _selection_start: Option<(usize, usize)>, _selection_end: Option<(usize, usize)>) -> Element<'a, Message> {
        // Use raw terminal mode for TUI apps (vim, top, etc.), block mode for normal shell
        if alt_screen_active {
            Canvas::new(TerminalCanvas {
                screen: screen.clone(),
                cell_width: 8.0,
                cell_height: 16.0,
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        } else {
            self.render_blocks(history, current, current_command)
        }
    }

    fn render_blocks<'a>(&self, history: &'a [Block], current: &'a Option<Block>, current_command: &'a str) -> Element<'a, Message> {
        let mut column = Column::new().spacing(5).padding(15);

        // Render history blocks
        for (index, block) in history.iter().enumerate() {
            let block_widget = self.render_block(block, index);
            column = column.push(block_widget);
        }

        // Render current block if running
        if let Some(block) = current {
            let current_block_widget = self.render_current_block(block);
            column = column.push(current_block_widget);
        }

        let scrollable = Scrollable::new(column)
            .width(Length::Fill)
            .height(Length::Fill);

        // Command input area with better styling
        let input = TextInput::new("", current_command)
            .on_input(Message::TerminalInput)
            .on_submit(Message::TerminalSubmit)
            .padding(12)
            .size(14.0)
            .font(Font::MONOSPACE);

        let input_container = Container::new(input)
            .width(Length::Fill)
            .padding(0);

        Column::new()
            .push(scrollable)
            .push(input_container)
            .height(Length::Fill)
            .into()
    }

    fn render_block<'a>(&self, block: &'a Block, index: usize) -> Element<'a, Message> {
        let (status_symbol, status_detail, _status_color) = match block.exit_code {
            Some(0) => ("✓".to_string(), String::new(), Color::from_rgb(0.2, 0.8, 0.2)),
            Some(code) => ("✗".to_string(), format!(" {}", code), Color::from_rgb(0.9, 0.3, 0.3)),
            None => ("⋯".to_string(), String::new(), Color::from_rgb(0.6, 0.6, 0.6)),
        };
        let status_display = format!("{}{}", status_symbol, status_detail);

        let duration_text = block.duration_ms
            .map(|ms| format!("{:.2}s", ms as f64 / 1000.0))
            .unwrap_or_else(|| "...".to_string());

        // Command line with prompt symbol
        let prompt = Text::new("❯ ")
            .font(Font::MONOSPACE)
            .size(14.0);
        
        let command = Text::new(&block.command)
            .font(Font::MONOSPACE)
            .size(14.0);

        let status = Text::new(status_display)
            .size(12.0);

        let duration = Text::new(duration_text)
            .size(11.0);

        let header = Row::new()
            .push(prompt)
            .push(command)
            .push(Container::new(Row::new()
                .push(status)
                .push(Text::new(" "))
                .push(duration)
                .spacing(5))
                .width(Length::Shrink))
            .spacing(8)
            .align_items(Alignment::Center);

        let buttons = Row::new()
            .push(Button::new(Text::new("Copy").size(11.0)).on_press(Message::CopyCommand(index)))
            .push(Button::new(Text::new("Rerun").size(11.0)).on_press(Message::RerunCommand(index)))
            .push(Button::new(Text::new(if block.collapsed { "Show" } else { "Hide" }).size(11.0)).on_press(Message::ToggleCollapsed(index)))
            .push(Button::new(Text::new(if block.pinned { "📌" } else { "Pin" }).size(11.0)).on_press(Message::TogglePin(index)))
            .spacing(5);

        let header_row = Row::new()
            .push(Container::new(header).width(Length::Fill))
            .push(buttons)
            .align_items(Alignment::Center)
            .spacing(10);

        let mut column = Column::new()
            .push(header_row)
            .spacing(8)
            .padding(12);

        if !block.collapsed && !block.output.is_empty() {
            let output_text = Text::new(&block.output)
                .font(Font::MONOSPACE)
                .size(13.0);
            let output_container = Container::new(output_text)
                .padding(8);
            column = column.push(output_container);
        }

        Container::new(column)
            .width(Length::Fill)
            .padding(10)
            .into()
    }

    fn render_current_block<'a>(&self, block: &'a Block) -> Element<'a, Message> {
        let duration_text = block.started_at
            .map(|start| format!("{:.2}s", (Utc::now() - start).num_milliseconds() as f64 / 1000.0))
            .unwrap_or_else(|| "...".to_string());

        // Command line with prompt symbol
        let prompt = Text::new("❯ ")
            .font(Font::MONOSPACE)
            .size(14.0);
        
        let command = Text::new(&block.command)
            .font(Font::MONOSPACE)
            .size(14.0);

        let status = Text::new("⏳")
            .size(12.0);

        let duration = Text::new(duration_text)
            .size(11.0);

        let header = Row::new()
            .push(prompt)
            .push(command)
            .push(Container::new(Row::new()
                .push(status)
                .push(Text::new(" "))
                .push(duration)
                .spacing(5))
                .width(Length::Shrink))
            .spacing(8)
            .align_items(Alignment::Center);

        let mut column = Column::new()
            .push(header)
            .spacing(8)
            .padding(12);

        if !block.output.is_empty() {
            let output_text = Text::new(&block.output)
                .font(Font::MONOSPACE)
                .size(13.0);
            let output_container = Container::new(output_text)
                .padding(8);
            column = column.push(output_container);
        }

        Container::new(column)
            .width(Length::Fill)
            .padding(10)
            .into()
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

                    // Draw background
                    let bg = bgcolor_to_iced(cell.bgcolor());
                    frame.fill_rectangle(Point::new(x, y), Size::new(self.cell_width, self.cell_height), bg);

                    // Draw text only if content is not empty
                    let content = cell.contents();
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