use iced::{Application, Command, Element, Settings, Subscription, Theme, time, window};

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
    Resize(u32, u32),
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
            Message::Resize(width, height) => {
                let (cell_w, cell_h) = self.renderer.cell_size();
                let cols = (width as f32 / cell_w) as u16;
                let rows = (height as f32 / cell_h) as u16;
                self.parser.resize(rows, cols);
                self.pty.resize(rows, cols).ok();
                Command::none()
            }
            Message::PtyData(_) | Message::None => Command::none(),
        }
    }

    fn view(&self) -> Element<Message> {
        self.renderer.view(self.parser.screen())
    }

    fn subscription(&self) -> Subscription<Message> {
        let time_sub = time::every(std::time::Duration::from_millis(10)).map(|_| Message::Tick);
        let resize_sub = iced::subscription::events_with(|event, _status| {
            match event {
                iced::Event::Window(window::Event::Resized { width, height }) => Some(Message::Resize(width, height)),
                _ => None,
            }
        });
        Subscription::batch(vec![time_sub, resize_sub])
    }
}

fn main() -> iced::Result {
    Tant::run(Settings::default())
}
