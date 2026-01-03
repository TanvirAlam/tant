// Renderer + UI shell

use iced::Element;

use crate::Message;

pub struct TerminalRenderer;

impl TerminalRenderer {
    pub fn new() -> Self {
        TerminalRenderer
    }

    pub fn view(&self, screen: &Screen) -> Element<Message> {
        // Placeholder: display screen size
        iced::widget::text(format!("Terminal: {} rows x {} cols", screen.size().0, screen.size().1)).into()
    }
}