// Terminal parser (escape sequences)
// Use vt100 for parsing

use vt100::Parser;

#[derive(Debug, Clone)]
pub enum ParserEvent {
    CommandStart,
    Command(String),
    CommandEnd(i32),
}

pub struct TerminalParser {
    parser: Parser,
    events: Vec<ParserEvent>,
}

impl TerminalParser {
    pub fn new(rows: u16, cols: u16) -> Self {
        let parser = Parser::new(rows, cols, 0);
        TerminalParser { parser, events: vec![] }
    }

    pub fn process(&mut self, data: &[u8]) {
        // Scan for OSC 1337 sequences
        let data_str = String::from_utf8_lossy(data);
        let mut i = 0;
        while i < data_str.len() {
            if data_str[i..].starts_with("\x1b]1337;") {
                let start = i + 7; // after \e]1337;
                if let Some(semi) = data_str[start..].find(';') {
                    let cmd = &data_str[start..start+semi];
                    if cmd == "CommandStart" {
                        self.events.push(ParserEvent::CommandStart);
                    } else if cmd == "Command" {
                        let cmd_start = start + semi + 1;
                        if let Some(end) = data_str[cmd_start..].find('\x1b') {
                            let command = data_str[cmd_start..cmd_start+end].to_string();
                            self.events.push(ParserEvent::Command(command));
                            i += cmd_start + end;
                            continue;
                        }
                    } else if cmd.starts_with("CommandEnd;") {
                        if let Some(status_str) = cmd.strip_prefix("CommandEnd;") {
                            if let Ok(status) = status_str.parse::<i32>() {
                                self.events.push(ParserEvent::CommandEnd(status));
                            }
                        }
                    }
                    // Skip to end of OSC
                    if let Some(end) = data_str[start..].find('\x1b') {
                        i += start + end;
                    } else {
                        break;
                    }
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
        self.parser.process(data);
    }

    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.parser.set_size(rows, cols);
    }

    pub fn take_events(&mut self) -> Vec<ParserEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn screen_text(&self) -> String {
        let mut text = String::new();
        for row in 0..self.parser.screen().size().1 as usize {
            for col in 0..self.parser.screen().size().0 as usize {
                if let Some(cell) = self.parser.screen().cell(row as u16, col as u16) {
                    text.push_str(&cell.contents());
                }
            }
            text.push('\n');
        }
        text
    }
}