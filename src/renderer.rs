// Renderer + UI shell

use iced::widget::{Column, Text, TextInput, Button, Row};
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

    pub fn view<'a>(&self, history: &'a [Block], current: &Option<Block>, current_command: &str, search_query: &str) -> Element<'a, Message> {
        let mut column = Column::new().spacing(10).padding(10);

        // Search input
        let search_input = TextInput::new("Search", search_query)
            .on_input(Message::UpdateSearch);
        column = column.push(search_input);

        // Filter history
        let filtered: Vec<(usize, &Block)> = history.iter().enumerate()
            .filter(|(_, b)| search_query.is_empty() || b.command.contains(search_query) || b.output.contains(search_query))
            .collect();

        for (orig_i, block) in filtered {
            let command_input = TextInput::new("", &block.command)
                .on_input(move |s| Message::UpdateCommand(orig_i, s));
            let rerun_button = Button::new("Run").on_press(Message::RerunCommand(orig_i));
            let pin_button = Button::new(if block.pinned { "Unpin" } else { "Pin" })
                .on_press(Message::TogglePin(orig_i));
            let command_row = Row::new().spacing(10)
                .push(command_input)
                .push(rerun_button)
                .push(pin_button);
            let status = Text::new(format!("Status: {:?}, Dir: {}, Branch: {:?}", block.status, block.directory, block.git_branch));
            let duration = Text::new(format!("Duration: {:?}", block.duration));
            let output = Text::new(&block.output);
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