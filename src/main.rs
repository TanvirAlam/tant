use iced::{Application, Command, Element, Settings, Subscription, Theme, time, window};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

mod pty;
mod parser;
mod renderer;

use parser::{TerminalParser, ParserEvent};
use renderer::TerminalRenderer;
use pty::PtyManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub command: String,
    pub output: String,
    pub status: Option<i32>,
    #[serde(skip, default)]
    pub start_time: Option<std::time::Instant>,
    #[serde(skip, default)]
    pub duration: Option<std::time::Duration>,
    pub directory: String,
    pub git_branch: Option<String>,
    pub host: String,
    pub pinned: bool,
}

pub struct Pane {
    pub pty: Arc<TokioMutex<PtyManager>>,
    pub parser: TerminalParser,
    pub history: Vec<Block>,
    pub current_block: Option<Block>,
    pub current_command: String,
    pub working_directory: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SerializablePane {
    pub history: Vec<Block>,
    pub current_command: String,
    pub working_directory: String,
}

pub struct Tab {
    pub panes: Vec<Pane>,
    pub active_pane: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SerializableTab {
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

impl Pane {
    pub fn new(shell: &str, working_directory: Option<String>) -> Result<Self, Box<dyn std::error::Error>> {
        let wd = working_directory.unwrap_or_else(|| std::env::current_dir().unwrap().to_string_lossy().to_string());
        let pty = PtyManager::new_with_cwd(shell, std::path::PathBuf::from(&wd))?;
        let parser = TerminalParser::new(24, 80);
        Ok(Pane {
            pty: Arc::new(TokioMutex::new(pty)),
            parser,
            history: vec![],
            current_block: None,
            current_command: String::new(),
            working_directory: wd,
        })
    }
}


#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    KeyPress(char),
    PtyData(Vec<u8>),
    Resize(u32, u32),
    ParserEvents(Vec<ParserEvent>),
    UpdateCommand(usize, String),
    RerunCommand(usize),
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
    fn save_session(&self) -> Result<(), Box<dyn std::error::Error>> {
        let serializable_tabs: Vec<SerializableTab> = self.layout.iter().map(|tab| {
            SerializableTab {
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
                        data.push_str(&format!("Command: {}\nOutput: {}\nStatus: {:?}\n\n", block.command, block.output, block.status));
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
        let (layout, active_tab) = if let Ok(saved_layout) = Self::load_session() {
            // Restore from saved session
            let mut tabs = vec![];
            for saved_tab in saved_layout.tabs {
                let mut panes = vec![];
                for saved_pane in saved_tab.panes {
                    let pane = Pane::new("bash", Some(saved_pane.working_directory.clone())).unwrap();
                    // Restore history and current_command
                    let mut pane = pane;
                    pane.history = saved_pane.history;
                    pane.current_command = saved_pane.current_command;
                    pane.working_directory = saved_pane.working_directory;
                    panes.push(pane);
                }
                let tab = Tab { panes, active_pane: saved_tab.active_pane };
                tabs.push(tab);
            }
            (tabs, saved_layout.active_tab)
        } else {
            // Default
            let pane = Pane::new("bash", None).unwrap();
            let tab = Tab { panes: vec![pane], active_pane: 0 };
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
        (Tant { layout, active_tab, renderer, search_query: String::new(), ai_settings, ai_response: None }, Command::none())
    }

    fn title(&self) -> String {
        "Tant Terminal".to_string()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Tick => {
                let _rt = tokio::runtime::Handle::current();
                for tab in &mut self.layout {
                    for pane in &mut tab.panes {
                        // Dummy: no PTY reading
                        // Simulate some input for demo
                        // pane.parser.process(b"hello\n");
                        // Handle parser events
                        let events = pane.parser.take_events();
                        for event in events {
                            match event {
                                ParserEvent::CommandStart => {
                                    if let Some(mut block) = pane.current_block.take() {
                                        if let Some(start) = block.start_time {
                                            block.duration = Some(start.elapsed());
                                        }
                                        pane.history.push(block);
                                    }
                                    pane.current_block = Some(Block {
                                        command: String::new(),
                                        output: String::new(),
                                        status: None,
                                        start_time: Some(std::time::Instant::now()),
                                        duration: None,
                                        directory: pane.working_directory.clone(),
                                        git_branch: None,
                                        host: "localhost".to_string(), // TODO: get actual host
                                        pinned: false,
                                    });
                                }
                                ParserEvent::Command(cmd) => {
                                    if let Some(ref mut block) = pane.current_block {
                                        block.command = cmd;
                                    }
                                }
                                ParserEvent::Directory(dir) => {
                                    if let Some(ref mut block) = pane.current_block {
                                        block.directory = dir.clone();
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
                                        block.status = Some(status);
                                        if let Some(start) = block.start_time {
                                            block.duration = Some(start.elapsed());
                                        }
                                        block.output = pane.parser.screen_text();
                                        pane.history.push(block);
                                    }
                                }
                            }
                        }
                    }
                }
                Command::none()
            }
            Message::KeyPress(c) => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                        // Dummy: no PTY writing
                        // Simulate processing input
                        let data = vec![c as u8];
                        pane.parser.process(&data);
                    }
                }
                Command::none()
            }
            Message::Resize(width, height) => {
                let (cell_w, cell_h) = self.renderer.cell_size();
                let cols = (width as f32 / cell_w) as u16;
                let rows = (height as f32 / cell_h) as u16;
                for tab in &mut self.layout {
                    for pane in &mut tab.panes {
                        pane.parser.resize(rows, cols);
                        // pane.pty.resize(rows, cols).ok();
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
                            let cmd = format!("{}\n", block.command);
                            // Dummy: process as input
                            pane.parser.process(cmd.as_bytes());
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
            Message::PtyData(_) | Message::ParserEvents(_) | Message::None => Command::none(),
        }
    }

    fn view(&self) -> Element<Message> {
        if let Some(tab) = self.layout.get(self.active_tab) {
            if let Some(pane) = tab.panes.get(tab.active_pane) {
                self.renderer.view(&pane.history, &pane.current_block, &pane.current_command, &self.search_query, pane.parser.screen(), &self.ai_settings, &self.ai_response)
            } else {
                let dummy_parser = TerminalParser::new(24, 80);
                self.renderer.view(&[], &None, &String::new(), &self.search_query, dummy_parser.screen(), &self.ai_settings, &self.ai_response)
            }
        } else {
            let dummy_parser = TerminalParser::new(24, 80);
            self.renderer.view(&[], &None, &String::new(), &self.search_query, dummy_parser.screen(), &self.ai_settings, &self.ai_response)
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let time_sub = time::every(std::time::Duration::from_millis(10)).map(|_| Message::Tick);
        let resize_sub = iced::event::listen().map(|event| {
            match event {
                iced::Event::Window(_, window::Event::Resized { width, height }) => Message::Resize(width, height),
                _ => Message::None,
            }
        });
        Subscription::batch(vec![time_sub, resize_sub])
    }
}

fn main() -> iced::Result {
    Tant::run(Settings::default())
}
