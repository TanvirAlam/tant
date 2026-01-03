// Renderer + UI shell

use iced::Element;
use vt100::Screen;

use crate::Message;

pub struct TerminalRenderer;

impl TerminalRenderer {
    pub fn new() -> Self {
        TerminalRenderer
    }

    pub fn view(&self, screen: &Screen) -> Element<Message> {
        let mut text = String::new();
        for row in 0..screen.size().1 {
            for col in 0..screen.size().0 {
                if let Some(cell) = screen.cell(row, col) {
                    text.push_str(&cell.contents());
                }
            }
            text.push('\n');
        }
        iced::widget::text(text).into()
    }
}