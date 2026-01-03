// Renderer + UI shell

use iced::widget::{Column, Text, Collapsible, TextInput, Button, Row};
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

    pub fn view(&self, history: &Vec<Block>, current: &Option<Block>, current_command: &str, _screen: &vt100::Screen) -> Element<Message> {
        let mut column = Column::new().spacing(10).padding(10);

        for (i, block) in history.iter().enumerate() {
            let command_input = TextInput::new("", &block.command)
                .on_input(move |s| Message::UpdateCommand(i, s));
            let rerun_button = Button::new("Run").on_press(Message::RerunCommand(i));
            let command_row = Row::new().spacing(10).push(command_input).push(rerun_button);
            let status = Text::new(format!("Status: {:?}", block.status));
            let duration = Text::new(format!("Duration: {:?}", block.duration));
            let output = Collapsible::new(
                Text::new("Output"),
                Text::new(&block.output),
            );
            column = column.push(command_row).push(status).push(duration).push(output);
        }

        if let Some(block) = current {
            let command = Text::new(format!("Current Command: {}", block.command));
            column = column.push(command);
        }

        let current_input = TextInput::new("Enter command", current_command)
            .on_input(Message::UpdateCurrent)
            .on_submit(Message::RunCurrent);
        column = column.push(current_input);

        column.into()
    }
}