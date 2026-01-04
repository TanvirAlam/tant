use iced::{Application, Command, Element, Settings, Subscription, Theme, time, window, mouse, clipboard, Point, Length, Color};
use iced::keyboard::{self, Key, Modifiers};
use iced::widget::{Row, Column, container};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use chrono::{DateTime, Utc};

mod pty;
mod parser;
mod renderer;

use parser::{TerminalParser, ParserEvent};
use renderer::TerminalRenderer;
use pty::PtyManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub command: String,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub exit_code: Option<i32>,
    pub cwd: Option<std::path::PathBuf>,
    pub output_range: Option<(usize, usize)>,
    pub pinned: bool,
    pub tags: Vec<String>,
    // Keep output for now, until shared store is implemented
    pub output: String,
    pub git_branch: Option<String>,
    pub host: String,
    #[serde(default)]
    pub collapsed: bool,
}

pub struct Pane {
    pub pty: Arc<TokioMutex<PtyManager>>,
    pub parser: TerminalParser,
    pub history: Vec<Block>,
    pub current_block: Option<Block>,
    pub current_command: String,
    pub working_directory: String,
    pub data_receiver: tokio::sync::mpsc::Receiver<Vec<u8>>,
    pub scroll_offset: usize,
    pub follow_mode: bool,
    pub selection_start: Option<(usize, usize)>,
    pub selection_end: Option<(usize, usize)>,
    pub mouse_button_down: bool,
    pub last_cursor_pos: Point,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SerializablePane {
    pub history: Vec<Block>,
    pub current_command: String,
    pub working_directory: String,
}

pub struct Tab {
    pub root: LayoutNode,
    pub panes: Vec<Pane>,
    pub active_pane: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SerializableTab {
    pub root: LayoutNode,
    pub panes: Vec<SerializablePane>,
    pub active_pane: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Layout {
    pub tabs: Vec<SerializableTab>,
    pub active_tab: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSettings {
    pub enabled: bool,
    pub send_current_command: bool,
    pub send_last_n_blocks: usize,
    pub send_repo_context: bool,
    pub provider: String, // e.g., "openai", "anthropic"
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayoutNode {
    Split {
        axis: Axis,
        ratio: f32,
        left: Box<LayoutNode>,
        right: Box<LayoutNode>,
    },
    Leaf {
        pane_id: usize,
    },
}

impl Pane {
    pub fn new(shell: &str, working_directory: Option<String>) -> Result<Self, Box<dyn std::error::Error>> {
        let wd = working_directory.unwrap_or_else(|| std::env::current_dir().unwrap().to_string_lossy().to_string());
        let (sender, receiver) = tokio::sync::mpsc::channel(100); // Buffer size
        let pty = PtyManager::new_with_cwd(shell, std::path::PathBuf::from(&wd))?;
        pty.spawn_reader(sender);
        let parser = TerminalParser::new(24, 80);
        Ok(Pane {
            pty: Arc::new(TokioMutex::new(pty)),
            parser,
            history: vec![],
            current_block: None,
            current_command: String::new(),
            working_directory: wd,
            data_receiver: receiver,
            scroll_offset: 0,
            follow_mode: true,
            selection_start: None,
            selection_end: None,
            mouse_button_down: false,
            last_cursor_pos: Point { x: 0.0, y: 0.0 },
        })
    }
}


#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    KeyPress(Key, Modifiers),
    TextInput(String),
    PtyData(Vec<u8>),
    Resize(u32, u32),
    ParserEvents(Vec<ParserEvent>),
    UpdateCommand(usize, String),
    RerunCommand(usize),
    CopyCommand(usize),
    ToggleCollapsed(usize),
    UpdateCurrent(String),
    RunCurrent,
    UpdateSearch(String),
    TogglePin(usize),
    SaveSession,
    AiExplainError,
    AiSuggestFix,
    AiGenerateCommand,
    AiSummarizeOutput,
    AiResponse(String),
    ToggleAiEnabled,
    UpdateAiSettings(AiSettings),
    Paste(String),
    WindowFocused,
    WindowUnfocused,
    TerminalInput(String),
    TerminalSubmit,
    MouseWheel(mouse::ScrollDelta),
    MouseButtonPressed(mouse::Button),
    MouseCursorMoved(Point),
    MouseButtonReleased(mouse::Button),
    CopySelected,
    None,
}

struct Tant {
    layout: Vec<Tab>,
    active_tab: usize,
    renderer: TerminalRenderer,
    search_query: String,
    ai_settings: AiSettings,
    ai_response: Option<String>,
}

impl Tant {
    fn key_to_bytes(key: &Key, modifiers: &Modifiers) -> Vec<u8> {
        match key {
            Key::Named(named_key) => {
                use iced::keyboard::key::Named;
                match named_key {
                    Named::Enter => vec![b'\r'],
                    Named::Backspace => vec![0x7f],
                    Named::Tab => vec![b'\t'],
                    Named::Escape => vec![0x1b],
                    Named::ArrowUp => b"\x1b[A".to_vec(),
                    Named::ArrowDown => b"\x1b[B".to_vec(),
                    Named::ArrowRight => b"\x1b[C".to_vec(),
                    Named::ArrowLeft => b"\x1b[D".to_vec(),
                    Named::Home => b"\x1b[H".to_vec(),
                    Named::End => b"\x1b[F".to_vec(),
                    Named::PageUp => b"\x1b[5~".to_vec(),
                    Named::PageDown => b"\x1b[6~".to_vec(),
                    Named::Delete => b"\x1b[3~".to_vec(),
                    Named::Insert => b"\x1b[2~".to_vec(),
                    _ => vec![],
                }
            }
            Key::Character(c) => {
                // Only handle Ctrl combinations here
                // Regular characters will be handled by event processing directly as text
                if modifiers.control() {
                    let ch = c.chars().next().unwrap_or('\0');
                    if ch.is_ascii_alphabetic() {
                        let upper = ch.to_ascii_uppercase();
                        // Ctrl+A = 0x01, Ctrl+B = 0x02, etc.
                        let code = (upper as u8) - b'A' + 1;
                        return vec![code];
                    } else if ch == ' ' {
                        return vec![0x00]; // Ctrl+Space = NUL
                    } else if ch == '@' {
                        return vec![0x00]; // Ctrl+@ = NUL
                    } else if ch == '[' {
                        return vec![0x1b]; // Ctrl+[ = ESC
                    } else if ch == '\\' {
                        return vec![0x1c]; // Ctrl+\\
                    } else if ch == ']' {
                        return vec![0x1d]; // Ctrl+]
                    } else if ch == '^' {
                        return vec![0x1e]; // Ctrl+^
                    } else if ch == '_' {
                        return vec![0x1f]; // Ctrl+_
                    }
                }
                // Don't send regular characters here - only special keys and modifiers
                vec![]
            }
            _ => vec![],
        }
    }

    fn save_session(&self) -> Result<(), Box<dyn std::error::Error>> {
        let serializable_tabs: Vec<SerializableTab> = self.layout.iter().map(|tab| {
            SerializableTab {
                root: tab.root.clone(),
                panes: tab.panes.iter().map(|pane| {
                    SerializablePane {
                        history: pane.history.clone(),
                        current_command: pane.current_command.clone(),
                        working_directory: pane.working_directory.clone(),
                    }
                }).collect(),
                active_pane: tab.active_pane,
            }
        }).collect();
        let layout = Layout {
            tabs: serializable_tabs,
            active_tab: self.active_tab,
        };
        let json = serde_json::to_string(&layout)?;
        std::fs::write("session.json", json)?;
        Ok(())
    }

    fn load_session() -> Result<Layout, Box<dyn std::error::Error>> {
        let json = std::fs::read_to_string("session.json")?;
        let layout: Layout = serde_json::from_str(&json)?;
        Ok(layout)
    }

    fn collect_ai_data(&self, include_current: bool, last_n: usize) -> String {
        let mut data = String::new();
        if let Some(tab) = self.layout.get(self.active_tab) {
            if let Some(pane) = tab.panes.get(tab.active_pane) {
                if include_current && self.ai_settings.send_current_command {
                    data.push_str(&format!("Current command: {}\n", pane.current_command));
                }
                if self.ai_settings.send_last_n_blocks > 0 {
                    let start = pane.history.len().saturating_sub(last_n);
                    for block in &pane.history[start..] {
                        data.push_str(&format!("Command: {}\nOutput: {}\nExit Code: {:?}\n\n", block.command, block.output, block.exit_code));
                    }
                }
                // Repo context not implemented yet
            }
        }
        data
    }


    fn call_ai(&self, prompt: &str, action: &str) -> String {
        // Mock AI response
        match action {
            "explain_error" => format!("AI Explanation: Based on the output, this error seems to be related to...\n\n{}", prompt),
            "suggest_fix" => format!("AI Suggestion: Try running the following command to fix the issue:\n\n```bash\nsome_fix_command\n```\n\n{}", prompt),
            "generate_command" => format!("AI Generated Command: Based on your request, try:\n\n```bash\ngenerated_command\n```\n\n{}", prompt),
            "summarize_output" => format!("AI Summary: The output shows...\n\n{}", prompt),
            _ => format!("AI Response: {}\n\n{}", action, prompt),
        }
    }
}

impl Application for Tant {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        let (layout, active_tab) = if let Ok(saved_layout) = Self::load_session() {
            // Restore from saved session
            let mut tabs = vec![];
            for saved_tab in saved_layout.tabs {
                let mut panes = vec![];
                for saved_pane in saved_tab.panes {
                    let pane = Pane::new(&shell, Some(saved_pane.working_directory.clone())).unwrap();
                    // Restore history and current_command
                    let mut pane = pane;
                    pane.history = saved_pane.history;
                    pane.current_command = saved_pane.current_command;
                    pane.working_directory = saved_pane.working_directory;
                    panes.push(pane);
                }
                let tab = Tab { root: saved_tab.root, panes, active_pane: saved_tab.active_pane };
                tabs.push(tab);
            }
            (tabs, saved_layout.active_tab)
        } else {
            // Default: single pane
            let pane = Pane::new(&shell, None).unwrap();
            let root = LayoutNode::Leaf { pane_id: 0 };
            let tab = Tab { root, panes: vec![pane], active_pane: 0 };
            (vec![tab], 0)
        };
        let renderer = TerminalRenderer::new();
        let ai_settings = AiSettings {
            enabled: false,
            send_current_command: false,
            send_last_n_blocks: 5,
            send_repo_context: false,
            provider: "mock".to_string(),
            api_key: None,
        };
        (
            Tant { layout, active_tab, renderer, search_query: String::new(), ai_settings, ai_response: None },
            window::gain_focus(window::Id::MAIN)
        )
    }

    fn title(&self) -> String {
        "Tant Terminal".to_string()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Tick => {
                let mut has_new_data = false;
                for tab in &mut self.layout {
                    for pane in &mut tab.panes {
                        // Receive data from the async reader
                        while let Ok(data) = pane.data_receiver.try_recv() {
                            eprintln!("Received {} bytes from PTY", data.len());
                            pane.parser.process(&data);
                            has_new_data = true;
                        }
                        // Handle parser events
                        let events = pane.parser.take_events();
                        for event in events {
                            match event {
                                ParserEvent::PromptShown => {
                                    // Prompt is being shown - this happens before user input
                                    // We can use this to prepare for the next command
                                    eprintln!("[Block Detection] Prompt shown");
                                }
                                ParserEvent::CommandStart => {
                                    if let Some(mut block) = pane.current_block.take() {
                                        if let Some(start) = block.started_at {
                                            block.ended_at = Some(Utc::now());
                                            block.duration_ms = Some((Utc::now() - start).num_milliseconds() as u64);
                                        }
                                        // Capture output at command end
                                        block.output = pane.parser.screen_text();
                                        pane.history.push(block);
                                    }
                                    // Clear screen text to start fresh for new command
                                    // Note: We can't actually clear the vt100 screen, but we'll capture the delta
                                    pane.current_block = Some(Block {
                                        command: String::new(),
                                        started_at: Some(Utc::now()),
                                        ended_at: None,
                                        duration_ms: None,
                                        exit_code: None,
                                        cwd: Some(std::path::PathBuf::from(&pane.working_directory)),
                                        output_range: None,
                                        pinned: false,
                                        tags: vec![],
                                        output: String::new(),
                                        git_branch: None,
                                        host: "localhost".to_string(), // TODO: get actual host
                                        collapsed: false,
                                    });
                                    eprintln!("[Block Detection] Command started - new block created");
                                }
                                ParserEvent::Command(cmd) => {
                                    if let Some(ref mut block) = pane.current_block {
                                        block.command = cmd;
                                    }
                                }
                                ParserEvent::Directory(dir) => {
                                    if let Some(ref mut block) = pane.current_block {
                                        block.cwd = Some(std::path::PathBuf::from(&dir));
                                        pane.working_directory = dir;
                                    }
                                }
                                ParserEvent::GitBranch(branch) => {
                                    if let Some(ref mut block) = pane.current_block {
                                        block.git_branch = Some(branch);
                                    }
                                }
                                ParserEvent::CommandEnd(status) => {
                                    if let Some(mut block) = pane.current_block.take() {
                                        block.exit_code = Some(status);
                                        if let Some(start) = block.started_at {
                                            block.ended_at = Some(Utc::now());
                                            block.duration_ms = Some((Utc::now() - start).num_milliseconds() as u64);
                                        }
                                        // Capture output - this gets the visible screen at command end
                                        block.output = pane.parser.screen_text();
                                        pane.history.push(block);
                                        eprintln!("[Block Detection] Command ended with status {} - block saved", status);
                                    }
                                }
                            }
                        }
                    }
                }
                // If follow mode and new data, scroll to bottom per pane
                if has_new_data {
                    for tab in &mut self.layout {
                        for pane in &mut tab.panes {
                            if pane.follow_mode {
                                pane.scroll_offset = 0;
                            }
                        }
                    }
                }
                Command::none()
            }
            Message::KeyPress(key, modifiers) => {
                eprintln!("KeyPress received: {:?} with modifiers: {:?}", key, modifiers);
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                        if let Ok(mut pty) = pane.pty.try_lock() {
                            let bytes = Self::key_to_bytes(&key, &modifiers);
                            eprintln!("Sending bytes: {:?}", bytes);
                            if !bytes.is_empty() {
                                pty.writer().write_all(&bytes).ok();
                                pty.writer().flush().ok();
                            }
                        }
                    }
                }
                Command::none()
            }
            Message::TextInput(text) => {
                eprintln!("TextInput received: {:?}", text);
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                        if let Ok(mut pty) = pane.pty.try_lock() {
                            pty.writer().write_all(text.as_bytes()).ok();
                            pty.writer().flush().ok();
                        }
                    }
                }
                Command::none()
            }
            Message::Paste(text) => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                        if let Ok(mut pty) = pane.pty.try_lock() {
                            // Bracketed paste mode
                            pty.writer().write_all(b"\x1b[200~").ok();
                            pty.writer().write_all(text.as_bytes()).ok();
                            pty.writer().write_all(b"\x1b[201~").ok();
                            pty.writer().flush().ok();
                        }
                    }
                }
                Command::none()
            }
            Message::Resize(width, height) => {
                let (cell_w, cell_h) = self.renderer.cell_size();
                let cols = (width as f32 / cell_w) as u16;
                let rows = (height as f32 / cell_h) as u16;
                let pixel_width = width as u16;
                let pixel_height = height as u16;
                for tab in &mut self.layout {
                    for pane in &mut tab.panes {
                        pane.parser.resize(rows, cols);
                        if let Ok(mut pty) = pane.pty.try_lock() {
                            pty.resize(rows, cols, pixel_width, pixel_height).ok();
                        }
                    }
                }
                Command::none()
            }
            Message::UpdateCommand(index, new_cmd) => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                        if let Some(block) = pane.history.get_mut(index) {
                            block.command = new_cmd;
                        }
                    }
                }
                Command::none()
            }
            Message::RerunCommand(index) => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                        if let Some(block) = pane.history.get(index) {
                            if let Ok(mut pty) = pane.pty.try_lock() {
                                let cmd = format!("{}\r", block.command);
                                pty.writer().write_all(cmd.as_bytes()).ok();
                                pty.writer().flush().ok();
                            }
                        }
                    }
                }
                Command::none()
            }
            Message::CopyCommand(index) => {
                if let Some(tab) = self.layout.get(self.active_tab) {
                    if let Some(pane) = tab.panes.get(tab.active_pane) {
                        if let Some(block) = pane.history.get(index) {
                            return clipboard::write(block.command.clone());
                        }
                    }
                }
                Command::none()
            }
            Message::ToggleCollapsed(index) => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                        if let Some(block) = pane.history.get_mut(index) {
                            block.collapsed = !block.collapsed;
                        }
                    }
                }
                Command::none()
            }
            Message::UpdateCurrent(cmd) => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                        pane.current_command = cmd;
                    }
                }
                Command::none()
            }
            Message::RunCurrent => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                        let cmd = format!("{}\n", pane.current_command);
                        // Dummy: process as input
                        pane.parser.process(cmd.as_bytes());
                        pane.current_command.clear();
                    }
                }
                Command::none()
            }
            Message::UpdateSearch(query) => {
                self.search_query = query;
                Command::none()
            }
            Message::TogglePin(index) => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                        if let Some(block) = pane.history.get_mut(index) {
                            block.pinned = !block.pinned;
                        }
                    }
                }
                Command::none()
            }
            Message::SaveSession => {
                self.save_session().ok();
                Command::none()
            }
            Message::AiExplainError => {
                if self.ai_settings.enabled {
                    let data = self.collect_ai_data(true, self.ai_settings.send_last_n_blocks);
                    let response = self.call_ai(&data, "explain_error");
                    self.ai_response = Some(response);
                }
                Command::none()
            }
            Message::AiSuggestFix => {
                if self.ai_settings.enabled {
                    let data = self.collect_ai_data(true, self.ai_settings.send_last_n_blocks);
                    let response = self.call_ai(&data, "suggest_fix");
                    self.ai_response = Some(response);
                }
                Command::none()
            }
            Message::AiGenerateCommand => {
                if self.ai_settings.enabled {
                    let data = self.collect_ai_data(false, 0);
                    let response = self.call_ai(&data, "generate_command");
                    self.ai_response = Some(response);
                }
                Command::none()
            }
            Message::AiSummarizeOutput => {
                if self.ai_settings.enabled {
                    let data = self.collect_ai_data(false, self.ai_settings.send_last_n_blocks);
                    let response = self.call_ai(&data, "summarize_output");
                    self.ai_response = Some(response);
                }
                Command::none()
            }
            Message::AiResponse(_) => Command::none(),
            Message::ToggleAiEnabled => {
                self.ai_settings.enabled = !self.ai_settings.enabled;
                Command::none()
            }
            Message::UpdateAiSettings(_) => Command::none(),
            Message::WindowFocused => {
                eprintln!("Window focused!");
                Command::none()
            }
            Message::WindowUnfocused => {
                eprintln!("Window unfocused!");
                Command::none()
            }
            Message::TerminalInput(input) => {
                // Update the current command text
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                        pane.current_command = input;
                    }
                }
                Command::none()
            }
            Message::TerminalSubmit => {
                // Send the current command to PTY with enter
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                        if let Ok(mut pty) = pane.pty.try_lock() {
                            let cmd = format!("{}\r", pane.current_command);
                            pty.writer().write_all(cmd.as_bytes()).ok();
                            pty.writer().flush().ok();
                            pane.current_command.clear();
                        }
                    }
                }
                Command::none()
            }
            Message::MouseWheel(delta) => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                        match delta {
                            mouse::ScrollDelta::Lines { y, .. } => {
                                if y > 0.0 {
                                    // Scroll up (show older content)
                                    if pane.scroll_offset < usize::MAX / 2 { // Prevent overflow
                                        pane.scroll_offset += y.abs() as usize;
                                        pane.follow_mode = false;
                                    }
                                } else if y < 0.0 {
                                    // Scroll down (show newer content)
                                    if pane.scroll_offset > 0 {
                                        pane.scroll_offset = pane.scroll_offset.saturating_sub(y.abs() as usize);
                                        if pane.scroll_offset == 0 {
                                            pane.follow_mode = true;
                                        }
                                    }
                                }
                            }
                            mouse::ScrollDelta::Pixels { y, .. } => {
                                // Convert pixels to lines, assuming cell_height
                                let lines = (y.abs() / 16.0) as usize;
                                if y > 0.0 {
                                    if pane.scroll_offset < usize::MAX / 2 {
                                        pane.scroll_offset += lines;
                                        pane.follow_mode = false;
                                    }
                                } else if y < 0.0 && pane.scroll_offset > 0 {
                                    pane.scroll_offset = pane.scroll_offset.saturating_sub(lines);
                                    if pane.scroll_offset == 0 {
                                        pane.follow_mode = true;
                                    }
                                }
                            }
                        }
                    }
                }
                Command::none()
            }
            Message::MouseButtonPressed(button) => {
                if button == mouse::Button::Left {
                    if let Some(tab) = self.layout.get_mut(self.active_tab) {
                        if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                            pane.mouse_button_down = true;
                            let cell_w = self.renderer.cell_size().0;
                            let cell_h = self.renderer.cell_size().1;
                            let col = (pane.last_cursor_pos.x / cell_w) as usize;
                            let row = (pane.last_cursor_pos.y / cell_h) as usize;
                            pane.selection_start = Some((row, col));
                            pane.selection_end = Some((row, col));
                        }
                    }
                }
                Command::none()
            }
            Message::MouseCursorMoved(position) => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                        pane.last_cursor_pos = position;
                        if pane.mouse_button_down {
                            let cell_w = self.renderer.cell_size().0;
                            let cell_h = self.renderer.cell_size().1;
                            let col = (position.x / cell_w) as usize;
                            let row = (position.y / cell_h) as usize;
                            pane.selection_end = Some((row, col));
                        }
                    }
                }
                Command::none()
            }
            Message::MouseButtonReleased(button) => {
                if button == mouse::Button::Left {
                    if let Some(tab) = self.layout.get_mut(self.active_tab) {
                        if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                            pane.mouse_button_down = false;
                        }
                    }
                }
                Command::none()
            }
            Message::CopySelected => {
                if let Some(tab) = self.layout.get(self.active_tab) {
                    if let Some(pane) = tab.panes.get(tab.active_pane) {
                        if let (Some(start), Some(end)) = (pane.selection_start, pane.selection_end) {
                            let contents = pane.parser.screen().contents();
                            let lines: Vec<&str> = contents.lines().collect();
                            let mut selected = String::new();
                            let (start_line, start_col) = start.min(end);
                            let (end_line, end_col) = start.max(end);
                            for line_idx in start_line..=end_line {
                                if let Some(line) = lines.get(line_idx) {
                                    let start_c = if line_idx == start_line { start_col } else { 0 };
                                    let end_c = if line_idx == end_line { end_col } else { line.len() };
                                    if start_c < line.len() {
                                        let end_c = end_c.min(line.len());
                                        selected.push_str(&line[start_c..end_c]);
                                        if line_idx < end_line {
                                            selected.push('\n');
                                        }
                                    }
                                }
                            }
                            return clipboard::write(selected);
                        }
                    }
                }
                Command::none()
            }
            Message::PtyData(_) | Message::ParserEvents(_) | Message::None => Command::none(),
        }
    }

    fn view(&self) -> Element<Message> {
        let layout_view = if let Some(tab) = self.layout.get(self.active_tab) {
            self.build_layout_view(&tab.root, &tab.panes)
        } else {
            let dummy_parser = TerminalParser::new(24, 80);
            self.renderer.view(&[], &None, "", &self.search_query, dummy_parser.screen(), false, &self.ai_settings, &self.ai_response, 0, None, None)
        };

        layout_view
    }

    fn theme(&self) -> Theme {
        // Create a dark theme with high-contrast input
        Theme::custom(
            "Tant Dark".to_string(),
            iced::theme::Palette {
                background: Color::from_rgb(0.12, 0.12, 0.12),
                text: Color::from_rgb(1.0, 1.0, 1.0),  // Pure white text
                primary: Color::from_rgb(0.4, 0.7, 0.9),
                success: Color::from_rgb(0.2, 0.8, 0.2),
                danger: Color::from_rgb(0.9, 0.3, 0.3),
            },
        )
    }

    fn subscription(&self) -> Subscription<Message> {
        let time_sub = time::every(std::time::Duration::from_millis(10)).map(|_| Message::Tick);
        let event_sub = iced::event::listen().map(|event| {
            eprintln!("Event received: {:?}", event);
            match event {
                iced::Event::Window(_, window::Event::Resized { width, height }) => {
                    Message::Resize(width, height)
                }
                iced::Event::Window(_, window::Event::Focused) => {
                    Message::WindowFocused
                }
                iced::Event::Window(_, window::Event::Unfocused) => {
                    Message::WindowUnfocused
                }
                iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                    Message::MouseWheel(delta)
                }
                iced::Event::Mouse(mouse::Event::ButtonPressed(button)) => {
                    Message::MouseButtonPressed(button)
                }
                iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                    Message::MouseCursorMoved(position)
                }
                iced::Event::Mouse(mouse::Event::ButtonReleased(button)) => {
                    Message::MouseButtonReleased(button)
                }
                iced::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, text, .. }) => {
                    eprintln!("Keyboard event - key: {:?}, modifiers: {:?}, text: {:?}", key, modifiers, text);
                    
                    // Handle special keys and modifiers first
                    if modifiers.control() || modifiers.alt() || modifiers.logo() {
                        eprintln!("Sending as KeyPress (with modifiers)");
                        return Message::KeyPress(key.clone(), modifiers);
                    }
                    
                    // Check if it's a named key (arrows, enter, etc.) but NOT Space
                    if matches!(key, iced::keyboard::Key::Named(_)) {
                        eprintln!("Sending as KeyPress (named key)");
                        return Message::KeyPress(key.clone(), modifiers);
                    }
                    
                    // For regular characters (including space), prefer text field if available
                    if let Some(txt) = text {
                        if !txt.is_empty() {
                            eprintln!("Sending as TextInput: {:?}", txt);
                            return Message::TextInput(txt.to_string());
                        }
                    }
                    
                    // Special handling for space if text is empty
                    if matches!(&key, iced::keyboard::Key::Character(c) if c == " ") {
                        eprintln!("Sending space character");
                        return Message::TextInput(" ".to_string());
                    }
                    
                    // Fallback to key press
                    eprintln!("Sending as KeyPress (fallback) for key: {:?}", key);
                    Message::KeyPress(key.clone(), modifiers)
                }
                _ => Message::None,
            }
        });
        Subscription::batch(vec![time_sub, event_sub])
    }
}

impl Tant {
    fn build_layout_view<'a>(&'a self, node: &LayoutNode, panes: &'a [Pane]) -> Element<'a, Message> {
        match node {
            LayoutNode::Leaf { pane_id } => {
                if let Some(pane) = panes.get(*pane_id) {
                    self.renderer.view(&pane.history, &pane.current_block, &pane.current_command, &self.search_query, pane.parser.screen(), pane.parser.is_alt_screen_active(), &self.ai_settings, &self.ai_response, pane.scroll_offset, pane.selection_start, pane.selection_end)
                } else {
                    let dummy_parser = TerminalParser::new(24, 80);
                    self.renderer.view(&[], &None, "", &self.search_query, dummy_parser.screen(), false, &self.ai_settings, &self.ai_response, 0, None, None)
                }
            }
            LayoutNode::Split { axis, ratio, left, right } => {
                let left_view = self.build_layout_view(left, panes);
                let right_view = self.build_layout_view(right, panes);
                match axis {
                    Axis::Horizontal => {
                        Row::new()
                            .push(container(left_view).width(Length::FillPortion((ratio * 100.0) as u16)))
                            .push(container(right_view).width(Length::FillPortion(((1.0 - ratio) * 100.0) as u16)))
                            .height(Length::Fill)
                            .into()
                    }
                    Axis::Vertical => {
                        Column::new()
                            .push(container(left_view).height(Length::FillPortion((ratio * 100.0) as u16)))
                            .push(container(right_view).height(Length::FillPortion(((1.0 - ratio) * 100.0) as u16)))
                            .width(Length::Fill)
                            .into()
                    }
                }
            }
        }
    }
}

fn main() -> iced::Result {
    Tant::run(Settings {
        window: window::Settings {
            size: iced::Size::new(1024.0, 768.0),
            position: window::Position::default(),
            min_size: None,
            max_size: None,
            visible: true,
            resizable: true,
            decorations: true,
            transparent: false,
            level: window::Level::Normal,
            icon: None,
            platform_specific: Default::default(),
            exit_on_close_request: true,
        },
        ..Settings::default()
    })
}
