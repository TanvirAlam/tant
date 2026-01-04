// Terminal parser (escape sequences)
// Use vt100 for parsing

use vt100::Parser;

#[derive(Debug, Clone)]
pub enum ParserEvent {
    CommandStart,
    Command(String),
    CommandEnd(i32),
    Directory(String),
    GitBranch(String),
}

pub struct TerminalParser {
    parser: Parser,
    events: Vec<ParserEvent>,
    scroll_offset: usize,
    dirty: bool,
}

impl TerminalParser {
    pub fn new(rows: u16, cols: u16) -> Self {
        let parser = Parser::new(rows, cols, 1000); // Large scrollback
        TerminalParser { parser, events: vec![], scroll_offset: 0, dirty: true }
    }

    pub fn process(&mut self, data: &[u8]) {
        // Just process the data directly with vt100 for now
        // OSC 1337 sequence parsing can be added later if needed
        self.parser.process(data);
        self.dirty = true;
    }

    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.parser.set_size(rows, cols);
        self.dirty = true;
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
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }
    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset;
        self.dirty = true;
    }
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }
}