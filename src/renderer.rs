// Renderer + UI shell

use iced::widget::{Column, Text, Collapsible};
use iced::{Element, Length};

use crate::{Message, Block};

pub struct TerminalRenderer;

impl TerminalRenderer {
    pub fn new() -> Self {
        TerminalRenderer
    }

    pub fn cell_size(&self) -> (f32, f32) {
        (8.0, 16.0)
    }

    pub fn view(&self, history: &Vec<Block>, current: &Option<Block>, _screen: &vt100::Screen) -> Element<Message> {
        let mut column = Column::new().spacing(10).padding(10);

        for block in history {
            let command = Text::new(format!("Command: {}", block.command));
            let status = Text::new(format!("Status: {:?}", block.status));
            let duration = Text::new(format!("Duration: {:?}", block.duration));
            let output = Collapsible::new(
                Text::new("Output"),
                Text::new(&block.output),
            );
            column = column.push(command).push(status).push(duration).push(output);
        }

        if let Some(block) = current {
            let command = Text::new(format!("Current Command: {}", block.command));
            column = column.push(command);
        }

        column.into()
    }
}