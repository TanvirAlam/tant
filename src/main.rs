use iced::{Application, Command, Element, Settings, Theme};

mod pty;
mod parser;
mod renderer;

use pty::PtyManager;
use parser::TerminalParser;
use renderer::TerminalRenderer;

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    KeyPress(char),
    None,
}

struct Tant {
    pty: PtyManager,
    parser: TerminalParser,
    renderer: TerminalRenderer,
}

impl Application for Tant {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let pty = PtyManager::new("bash").unwrap();
        let parser = TerminalParser::new(24, 80);
        let renderer = TerminalRenderer::new();
        (Tant { pty, parser, renderer }, Command::none())
    }

    fn title(&self) -> String {
        "Tant Terminal".to_string()
    }

    fn update(&mut self, _message: Message) -> Command<Message> {
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        self.renderer.view(self.parser.screen())
    }
}

fn main() -> iced::Result {
    Tant::run(Settings::default())
}
