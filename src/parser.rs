// Terminal parser (escape sequences)
// Use vt100 for parsing

use vt100::Parser;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum ParserEvent {
    CommandStart,
    Command(String),
    CommandEnd(i32),
    Directory(String),
    GitInfo { branch: String, status: Option<GitStatus> },
    PromptShown,
}

// OSC 133 sequence markers (Warp/FinalTerm style)
// These are emitted by shell integration scripts
const OSC_PROMPT_START: &str = "\x1b]133;A";
const OSC_COMMAND_START: &str = "\x1b]133;C";
const OSC_COMMAND_END_PREFIX: &str = "\x1b]133;D";
const OSC_GIT_INFO_PREFIX: &str = "\x1b]133;G;";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GitStatus {
    Clean,
    Dirty,
    Conflicts,
}

pub struct TerminalParser {
    parser: Parser,
    events: Vec<ParserEvent>,
    dirty: bool,
    buffer: Vec<u8>, // Buffer for detecting OSC sequences
    in_command: bool,
    alt_screen_active: bool,
}

impl TerminalParser {
    pub fn new(rows: u16, cols: u16) -> Self {
        let parser = Parser::new(rows, cols, 1000); // Large scrollback
        TerminalParser {
            parser,
            events: vec![],
            dirty: true,
            buffer: Vec::new(),
            in_command: false,
            alt_screen_active: false,
        }
    }

    pub fn process(&mut self, data: &[u8]) {
        // Append to buffer for OSC sequence detection
        self.buffer.extend_from_slice(data);
        
        // Detect OSC 133 sequences
        self.detect_shell_integration_markers();
        
        // Process with vt100
        self.parser.process(data);
        self.dirty = true;
    }
    
    fn detect_shell_integration_markers(&mut self) {
        let buffer_str = String::from_utf8_lossy(&self.buffer);
        
        // Check for alt screen sequences
        if buffer_str.contains("\x1b[?1049h") {
            self.alt_screen_active = true;
            eprintln!("[Alt Screen] Entered alt screen mode");
        }
        if buffer_str.contains("\x1b[?1049l") {
            self.alt_screen_active = false;
            eprintln!("[Alt Screen] Exited alt screen mode");
        }
        
        // Check for OSC 133 sequences
        if buffer_str.contains(OSC_PROMPT_START) {
            self.events.push(ParserEvent::PromptShown);
            eprintln!("[Shell Integration] Prompt shown");
        }
        
        if buffer_str.contains(OSC_COMMAND_START) {
            self.events.push(ParserEvent::CommandStart);
            self.in_command = true;
            eprintln!("[Shell Integration] Command started");
        }
        
        // Check for command end with exit code
        // OSC 133;D;exit_code ESC\
        if let Some(pos) = buffer_str.find(OSC_COMMAND_END_PREFIX) {
            let rest = &buffer_str[pos + OSC_COMMAND_END_PREFIX.len()..];
            // Parse exit code from ;exit_code ESC\
            if let Some(end_pos) = rest.find('\x07').or_else(|| rest.find("\x1b\\")) {
                let params = &rest[..end_pos];
                if let Some(exit_code_str) = params.strip_prefix(';') {
                    if let Ok(exit_code) = exit_code_str.trim().parse::<i32>() {
                        self.events.push(ParserEvent::CommandEnd(exit_code));
                        self.in_command = false;
                        eprintln!("[Shell Integration] Command ended with exit code: {}", exit_code);
                    }
                }
            }
        }

        if let Some(pos) = buffer_str.rfind(OSC_GIT_INFO_PREFIX) {
            let rest = &buffer_str[pos + OSC_GIT_INFO_PREFIX.len()..];
            if let Some(end_pos) = rest.find('\x07').or_else(|| rest.find("\x1b\\")) {
                let payload = &rest[..end_pos];
                if let Some((branch, status)) = Self::parse_git_info(payload) {
                    self.events.push(ParserEvent::GitInfo { branch, status });
                    eprintln!("[Shell Integration] Git info detected");
                }
            }
        }
        
        // Limit buffer size to prevent unbounded growth
        if self.buffer.len() > 8192 {
            // Keep only the last 4KB
            let keep_from = self.buffer.len() - 4096;
            self.buffer = self.buffer[keep_from..].to_vec();
        }
    }

    fn parse_git_info(payload: &str) -> Option<(String, Option<GitStatus>)> {
        let mut branch: Option<String> = None;
        let mut status: Option<GitStatus> = None;
        for part in payload.split(';') {
            let (key, value) = part.split_once('=')?;
            match key {
                "branch" => {
                    if !value.is_empty() {
                        branch = Some(value.to_string());
                    }
                }
                "status" => {
                    status = match value {
                        "clean" => Some(GitStatus::Clean),
                        "dirty" => Some(GitStatus::Dirty),
                        "conflicts" => Some(GitStatus::Conflicts),
                        _ => None,
                    };
                }
                _ => {}
            }
        }
        branch.map(|branch| (branch, status))
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
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }
    
    pub fn is_alt_screen_active(&self) -> bool {
        self.alt_screen_active
    }
}
