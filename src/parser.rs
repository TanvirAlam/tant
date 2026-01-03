// Terminal parser (escape sequences)
// Use vt100 for parsing

use vt100::Parser;

pub struct TerminalParser {
    parser: Parser,
}

impl TerminalParser {
    pub fn new(rows: u16, cols: u16) -> Self {
        let parser = Parser::new(rows, cols, 0);
        TerminalParser { parser }
    }

    pub fn process(&mut self, data: &[u8]) {
        self.parser.process(data);
    }

    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.parser.set_size(rows, cols);
    }
}