use iced::{Application, Command, Element, Settings, Subscription, Theme, time};

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
    PtyData(Vec<u8>),
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

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Tick => {
                // Read from PTY synchronously
                let rt = tokio::runtime::Handle::current();
                let mut buf = vec![0u8; 1024];
                let n = rt.block_on(async {
                    use tokio::io::AsyncReadExt;
                    self.pty.reader().read(&mut buf).await.unwrap_or(0)
                });
                if n > 0 {
                    self.parser.process(&buf[..n]);
                }
                Command::none()
            }
            Message::KeyPress(c) => {
                let rt = tokio::runtime::Handle::current();
                let data = vec![c as u8];
                rt.block_on(async {
                    use tokio::io::AsyncWriteExt;
                    self.pty.writer().write_all(&data).await.ok();
                });
                Command::none()
            }
            Message::PtyData(_) | Message::None => Command::none(),
        }
    }

    fn view(&self) -> Element<Message> {
        self.renderer.view(self.parser.screen())
    }

    fn subscription(&self) -> Subscription<Message> {
        time::every(std::time::Duration::from_millis(10)).map(|_| Message::Tick)
    }
}

fn main() -> iced::Result {
    Tant::run(Settings::default())
}
