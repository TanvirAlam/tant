use iced::{Application, Command, Element, Settings, Subscription, Theme, time, window, mouse, clipboard, Point, Length, Color, Size, Rectangle, Border, Background};
use iced::keyboard::{self, Key, Modifiers};
use iced::widget::{Row, Column, container, TextInput, text_input, scrollable};
use iced::widget::scrollable::RelativeOffset;
use log::{debug, info, warn, error};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex as TokioMutex;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

mod pty;
mod parser;
mod renderer;
mod export;
mod themes;
mod ai;

use parser::{TerminalParser, ParserEvent, GitStatus};
use renderer::{TerminalRenderer, StyleRun};
use export::{AiConversationExport, AiConversationExportScope, AiConversationMessage, AiConversationMetadata, AiReferencedBlock, ExportFormat, format_ai_conversation_export, format_blocks, write_ai_export_file, write_export_file};
use themes::preset_theme;
use ai::{AiRequest, send_request};
use pty::PtyManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub font_family: String,
    pub font_size: f32,
    pub enable_ligatures: bool,
    pub padding: f32,
    pub line_height: f32,
    pub colors: HashMap<String, [f32; 3]>, // RGB values
}

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
    #[serde(default)]
    pub selected: bool,
    // Keep output for now, until shared store is implemented
    pub output: String,
    pub git_branch: Option<String>,
    pub git_status: Option<GitStatus>,
    pub host: String,
    #[serde(default)]
    pub is_remote: bool,
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
    pub title: String,
    pub ai_panel_open: bool,
    pub ai_context_scope: AiContextScope,
    pub ai_chat: Vec<AiChatMessage>,
    pub ai_input: String,
    pub ai_pending: bool,
    pub ai_streaming: bool,
    pub ai_stream_remaining: String,
    pub ai_stream_target: Option<usize>,
    pub ai_request_id: u64,
    pub ai_request_started_at: Option<DateTime<Utc>>,
    pub ai_request_chars: usize,
    pub ai_request_provider: Option<String>,
    pub ai_request_model: Option<String>,
    pub history_scroll_id: scrollable::Id,
    pub highlighted_block: Option<usize>,
    pub ai_redaction_override: bool,
    pub ai_last_redactions: Vec<String>,
    pub ai_last_redacted_preview: Option<String>,
    pub ai_selected_template: Option<AiPromptTemplateId>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SerializablePane {
    pub history: Vec<Block>,
    pub current_command: String,
    pub working_directory: String,
    pub title: String,
    pub scroll_offset: usize,
}

pub struct Tab {
    pub root: LayoutNode,
    pub panes: Vec<Pane>,
    pub active_pane: usize,
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SerializableTab {
    pub root: LayoutNode,
    pub panes: Vec<SerializablePane>,
    pub active_pane: usize,
    pub title: String,
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
    pub redact_secrets: bool,
    pub allow_sensitive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanTier {
    Free,
    Pro,
    Team,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanLimits {
    pub daily_messages: u32,
    pub daily_chars: u32,
    pub max_context_chars: usize,
    pub allow_streaming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingProfile {
    pub plan: PlanTier,
    pub customer_id: Option<String>,
    pub subscription_id: Option<String>,
    pub renewal_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub ai_onboarding_seen: bool,
    #[serde(default = "default_ai_share_link_enabled")]
    pub ai_share_link_enabled: bool,
    #[serde(default = "default_plan_tier")]
    pub plan_tier: PlanTier,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self { ai_onboarding_seen: false, ai_share_link_enabled: true, plan_tier: PlanTier::Free }
    }
}

fn default_ai_share_link_enabled() -> bool {
    true
}

fn default_plan_tier() -> PlanTier {
    PlanTier::Free
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEntry {
    pub timestamp: DateTime<Utc>,
    pub request_chars: usize,
    pub response_chars: usize,
    pub duration_ms: u64,
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageLedger {
    pub entries: Vec<UsageEntry>,
    pub last_reset: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSnapshot {
    pub used_messages: u32,
    pub used_chars: u32,
    pub remaining_messages: u32,
    pub remaining_chars: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionRuleConfig {
    pub label: String,
    pub pattern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionConfig {
    pub rules: Vec<RedactionRuleConfig>,
}

#[derive(Debug, Clone)]
struct RedactionRule {
    label: String,
    regex: regex::Regex,
}

#[derive(Debug, Clone)]
struct RedactionResult {
    redacted: String,
    matches: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AiContextScope {
    CurrentCommand,
    LastNBlocks,
    SelectedBlocks,
    EntireSession,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AiChatRole {
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub struct AiChatMessage {
    pub role: AiChatRole,
    pub content: String,
    pub sources: Vec<AiCitation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiCitation {
    pub block_index: Option<usize>,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AiQuickAction {
    ExplainError,
    SummarizeOutput,
    GenerateCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AiPromptTemplateId {
    ExplainError,
    DebugCommand,
    SummarizeOutput,
    GenerateCommand,
    RefactorSnippet,
}

#[derive(Debug, Clone)]
pub struct AiPromptTemplate {
    pub id: AiPromptTemplateId,
    pub label: &'static str,
    pub prompt: &'static str,
    pub tip: &'static str,
}

pub const AI_PROMPT_TEMPLATES: &[AiPromptTemplate] = &[
    AiPromptTemplate {
        id: AiPromptTemplateId::ExplainError,
        label: "Explain error",
        prompt: "Explain the error below and the most likely cause.\n\nError:\n",
        tip: "Tip: include the exact error message and the command you ran.",
    },
    AiPromptTemplate {
        id: AiPromptTemplateId::DebugCommand,
        label: "Debug command",
        prompt: "Help me debug this command. Explain what is failing and how to fix it.\n\nCommand:\n\nOutput:\n",
        tip: "Tip: add the full command output and any environment details.",
    },
    AiPromptTemplate {
        id: AiPromptTemplateId::SummarizeOutput,
        label: "Summarize output",
        prompt: "Summarize the output below in a few bullets.\n\nOutput:\n",
        tip: "Tip: include only the relevant section of output you want summarized.",
    },
    AiPromptTemplate {
        id: AiPromptTemplateId::GenerateCommand,
        label: "Generate command",
        prompt: "Generate a command that does the following task.\n\nTask:\n",
        tip: "Tip: specify OS, tools, and any constraints (e.g., no sudo).",
    },
    AiPromptTemplate {
        id: AiPromptTemplateId::RefactorSnippet,
        label: "Refactor snippet",
        prompt: "Refactor the code below for clarity and best practices.\n\nCode:\n",
        tip: "Tip: mention the language, style preferences, and any constraints.",
    },
];

pub fn get_ai_prompt_template(id: AiPromptTemplateId) -> Option<&'static AiPromptTemplate> {
    AI_PROMPT_TEMPLATES.iter().find(|template| template.id == id)
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone)]
pub enum PaletteAction {
    SplitPaneHorizontal,
    SplitPaneVertical,
    ClosePane,
    SwitchTab(usize),
    RunPinnedCommand(usize),
    ToggleAi,
    ToggleAiPanel,
    SetAiContextScope(AiContextScope),
    ApplyAiTemplate(AiPromptTemplateId),
    ExportTheme,
    ImportTheme,
    OpenBilling,
    // Add more as needed
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
            title: "Terminal".to_string(),
            ai_panel_open: false,
            ai_context_scope: AiContextScope::LastNBlocks,
            ai_chat: Vec::new(),
            ai_input: String::new(),
            ai_pending: false,
            ai_streaming: false,
            ai_stream_remaining: String::new(),
            ai_stream_target: None,
            ai_request_id: 0,
            ai_request_started_at: None,
            ai_request_chars: 0,
            ai_request_provider: None,
            ai_request_model: None,
            history_scroll_id: scrollable::Id::unique(),
            highlighted_block: None,
            ai_redaction_override: false,
            ai_last_redactions: Vec::new(),
            ai_last_redacted_preview: None,
            ai_selected_template: None,
        })
    }
}


#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    KeyPress(Key, Modifiers),
    KeyboardEvent(Key, Modifiers, Option<String>),
    TextInput(String),
    PtyData(Vec<u8>),
    Resize(u32, u32),
    ParserEvents(Vec<ParserEvent>),
    UpdateCommand(usize, String),
    RerunCommand(usize),
    CopyCommand(usize),
    CopyOutput(usize),
    ToggleBlockSelected(usize),
    SelectAllBlocks,
    DeselectAllBlocks,
    ExportBlock(usize, ExportFormat),
    ExportSelected(ExportFormat),
    CopyBlockExport(usize, ExportFormat),
    CopySelectedExport(ExportFormat),
    ToggleCollapsed(usize),
    UpdateCurrent(String),
    RunCurrent,
    UpdateSearch(String),
    FocusSearch,
    ToggleSearchSuccess,
    ToggleSearchFailure,
    ToggleSearchPinned,
    ClearSearch,
    TogglePin(usize),
    SaveSession,
    AiExplainError,
    AiSuggestFix,
    AiGenerateCommand,
    AiSummarizeOutput,
    AiResponse(String),
    AiResponseReceived(usize, u64, String),
    ToggleAiPanel,
    AiPanelInputChanged(usize, String),
    AiPanelSend(usize),
    AiPanelStop(usize),
    AiPanelSetScope(usize, AiContextScope),
    AiPanelQuickAction(usize, AiQuickAction),
    AiPanelHoverCitation(usize, Option<usize>),
    AiPanelJumpToBlock(usize, usize),
    AiPanelToggleRedactionOverride(usize, bool),
    AiPanelReloadRedactionRules,
    AiPanelOpenRedactionPreview(usize),
    AiPanelCloseRedactionPreview(usize),
    AiPanelExportConversation(usize, ExportFormat),
    AiPanelExportSession(ExportFormat),
    ToggleAiEnabled,
    UpdateAiSettings(AiSettings),
    AiTemplateSelected(usize, AiPromptTemplateId),
    AiOnboardingContinue,
    AiOnboardingSkip,
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
    OpenCommandPalette,
    CloseCommandPalette,
    UpdatePaletteQuery(String),
    UpdatePaletteSelection(isize),
    ExecutePaletteAction(PaletteAction),
    SplitPaneHorizontal,
    SplitPaneVertical,
    ClosePane,
    NewTab,
    CloseTab,
    CloseTabAt(usize),
    SelectTab(usize),
    PrevTab,
    NextTab,
    BeginRenameTab(usize),
    RenameTabInput(String),
    CommitRenameTab,
    CancelRenameTab,
    StartHistorySearch,
    UpdateHistorySearch(String),
    NextHistoryMatch,
    PrevHistoryMatch,
    ApplyHistoryMatch,
    CancelHistorySearch,
    SwitchPane(isize),
    AdjustSplitRatio(Axis, f32),
    ExportTheme,
    ImportTheme,
    OpenBilling,
    CloseBilling,
    BillingUpgradeRequested,
    BillingPlanSelected(PlanTier),
    BillingSyncRequested,
    None,
}

struct Tant {
    layout: Vec<Tab>,
    active_tab: usize,
    renderer: TerminalRenderer,
    search_query: String,
    search_success_only: bool,
    search_failure_only: bool,
    search_pinned_only: bool,
    search_input_id: text_input::Id,
    ai_settings: AiSettings,
    ai_response: Option<String>,
    app_config: AppConfig,
    ai_onboarding_open: bool,
    show_command_palette: bool,
    palette_query: String,
    palette_selected: usize,
    render_cache: Arc<Mutex<HashMap<(usize, usize, u16), Vec<StyleRun>>>>,
    row_hashes: Arc<Mutex<HashMap<(usize, usize, u16), u64>>>,
    theme_config: ThemeConfig,
    host_info: HostInfo,
    window_size: Size,
    resize_state: Option<SplitResizeState>,
    last_cursor_pos: Point,
    renaming_tab: Option<usize>,
    rename_buffer: String,
    history_search_active: bool,
    history_search_query: String,
    history_matches: Vec<String>,
    history_selected: usize,
    export_toast: Option<ExportToast>,
    usage_ledger: UsageLedger,
    billing_profile: BillingProfile,
    usage_snapshot: UsageSnapshot,
    show_billing: bool,
}

#[derive(Debug, Clone)]
pub struct ExportToast {
    message: String,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
pub struct AiContextPreview {
    pub block_count: usize,
    pub char_count: usize,
    pub token_estimate: usize,
}

#[derive(Debug, Clone)]
struct HostInfo {
    display: String,
    is_remote: bool,
}

#[derive(Debug, Clone, Copy)]
enum SplitDirection {
    Left,
    Right,
}

#[derive(Debug, Clone)]
struct SplitResizeState {
    path: Vec<SplitDirection>,
    rect: Rectangle,
}

fn is_remote_session() -> bool {
    std::env::var("SSH_CONNECTION").is_ok()
        || std::env::var("SSH_CLIENT").is_ok()
        || std::env::var("SSH_TTY").is_ok()
}

fn get_hostname() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .filter(|h| !h.trim().is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn get_username() -> Option<String> {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .ok()
        .filter(|u| !u.trim().is_empty())
}

fn resolve_host_info() -> HostInfo {
    let hostname = get_hostname();
    let is_remote = is_remote_session();
    let display = if is_remote {
        if let Some(user) = get_username() {
            format!("{}@{}", user, hostname)
        } else {
            hostname
        }
    } else {
        hostname
    };
    HostInfo { display, is_remote }
}

impl Tant {
    fn load_app_config() -> AppConfig {
        let path = std::path::Path::new("config.json");
        if let Ok(contents) = std::fs::read_to_string(path) {
            serde_json::from_str::<AppConfig>(&contents).unwrap_or_default()
        } else {
            AppConfig::default()
        }
    }

    fn save_app_config(&self) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self.app_config)?;
        std::fs::write("config.json", json)?;
        Ok(())
    }

    fn apply_ai_template(&mut self, pane_id: usize, template_id: AiPromptTemplateId, open_panel: bool) {
        if let Some(template) = get_ai_prompt_template(template_id) {
            if let Some(tab) = self.layout.get_mut(self.active_tab) {
                if let Some(pane) = tab.panes.get_mut(pane_id) {
                    pane.ai_input = template.prompt.to_string();
                    pane.ai_selected_template = Some(template_id);
                    if open_panel {
                        pane.ai_panel_open = true;
                    }
                }
            }
        }
    }

    fn open_ai_panel_for_active_pane(&mut self) {
        if let Some(tab) = self.layout.get_mut(self.active_tab) {
            if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                pane.ai_panel_open = true;
            }
        }
    }
    fn default_redaction_rules() -> Vec<RedactionRule> {
        let patterns = vec![
            ("AWS Access Key", r"\bAKIA[0-9A-Z]{16}\b"),
            ("AWS Secret Key", r"(?i)aws_secret_access_key\s*[:=]\s*[^\s]+"),
            ("GitHub Token", r"\bgh[pousr]_[A-Za-z0-9]{36,}\b"),
            ("Bearer Token", r"(?i)bearer\s+[A-Za-z0-9\-\._~\+\/]+=*"),
            ("Private Key Block", r"-----BEGIN (RSA|EC|OPENSSH|DSA|PGP) PRIVATE KEY-----[\s\S]+?-----END (RSA|EC|OPENSSH|DSA|PGP) PRIVATE KEY-----"),
        ];
        patterns
            .into_iter()
            .filter_map(|(label, pattern)| {
                regex::Regex::new(pattern).ok().map(|regex| RedactionRule { label: label.to_string(), regex })
            })
            .collect()
    }

    fn load_redaction_config(&self) -> RedactionConfig {
        let path = std::path::Path::new("redaction_rules.json");
        if let Ok(contents) = std::fs::read_to_string(path) {
            serde_json::from_str::<RedactionConfig>(&contents).unwrap_or(RedactionConfig { rules: Vec::new() })
        } else {
            RedactionConfig { rules: Vec::new() }
        }
    }

    fn build_redaction_rules(&self) -> Vec<RedactionRule> {
        let mut rules = Self::default_redaction_rules();
        let config = self.load_redaction_config();
        for rule in config.rules {
            if let Ok(regex) = regex::Regex::new(&rule.pattern) {
                rules.push(RedactionRule { label: rule.label, regex });
            }
        }
        rules
    }

    fn redact_prompt(&self, input: &str) -> RedactionResult {
        let rules = self.build_redaction_rules();
        let mut redacted = input.to_string();
        let mut matches = Vec::new();
        for rule in rules {
            if rule.regex.is_match(&redacted) {
                matches.push(rule.label.clone());
                redacted = rule.regex.replace_all(&redacted, "[REDACTED]").to_string();
            }
        }
        matches.sort();
        matches.dedup();
        RedactionResult { redacted, matches }
    }
    fn block_label(&self, index: usize, block: &Block) -> String {
        if let Some(started) = block.started_at {
            format!("block#{} ({})", index + 1, started.to_rfc3339())
        } else {
            format!("block#{}", index + 1)
        }
    }

    fn resolve_context_preview(&self, pane: &Pane, scope: AiContextScope) -> AiContextPreview {
        let (_context, _sources, preview) = self.build_ai_context(pane, scope);
        preview
    }

    fn build_ai_context(&self, pane: &Pane, scope: AiContextScope) -> (String, Vec<AiCitation>, AiContextPreview) {
        let mut sources = Vec::new();
        let mut context = String::new();
        let cwd = pane.working_directory.clone();
        let host = self.host_info.display.clone();
        context.push_str(&format!("Environment:\n- cwd: {}\n- host: {}\n", cwd, host));

        let blocks: Vec<(usize, &Block)> = match scope {
            AiContextScope::CurrentCommand => Vec::new(),
            AiContextScope::LastNBlocks => {
                let start = pane.history.len().saturating_sub(self.ai_settings.send_last_n_blocks.max(1));
                pane.history.iter().enumerate().skip(start).collect()
            }
            AiContextScope::SelectedBlocks => pane
                .history
                .iter()
                .enumerate()
                .filter(|(_, block)| block.selected)
                .collect(),
            AiContextScope::EntireSession => pane.history.iter().enumerate().collect(),
        };

        match scope {
            AiContextScope::CurrentCommand => {
                sources.push(AiCitation { block_index: None, label: "current_command".to_string() });
                context.push_str("\nCurrent command:\n");
                context.push_str(&pane.current_command);
                context.push('\n');
            }
            _ => {
                if blocks.is_empty() {
                    context.push_str("\nNo command blocks available for the selected scope.\n");
                } else {
                    context.push_str("\nCommand blocks:\n");
                    for (index, block) in blocks {
                        let label = self.block_label(index, block);
                        sources.push(AiCitation { block_index: Some(index), label: label.clone() });
                        context.push_str(&format!("\n[{}]\nCommand: {}\nExit: {:?}\nOutput:\n{}\n", label, block.command, block.exit_code, block.output));
                    }
                }
            }
        }

        let limits = Self::plan_limits(self.billing_profile.plan);
        let mut char_count = context.chars().count();
        if char_count > limits.max_context_chars {
            context = context
                .chars()
                .take(limits.max_context_chars)
                .collect::<String>();
            context.push_str("\n\n[context truncated for plan limits]\n");
            char_count = context.chars().count();
        }
        let token_estimate = (char_count / 4).max(1);
        let block_count = sources.iter().filter(|item| item.block_index.is_some()).count();
        let preview = AiContextPreview { block_count, char_count, token_estimate };

        (context, sources, preview)
    }

    fn build_ai_conversation_export(
        &self,
        scope: AiConversationExportScope,
        tab_index: usize,
        pane_index: Option<usize>,
    ) -> Option<AiConversationExport> {
        let tab = self.layout.get(tab_index)?;
        let (panes, pane_title, working_directory) = match scope {
            AiConversationExportScope::Pane => {
                let pane_id = pane_index?;
                let pane = tab.panes.get(pane_id)?;
                (
                    vec![(pane_id, pane)],
                    Some(pane.title.clone()),
                    Some(pane.working_directory.clone()),
                )
            }
            AiConversationExportScope::Session => {
                let wd = tab
                    .panes
                    .get(tab.active_pane)
                    .map(|pane| pane.working_directory.clone());
                (tab.panes.iter().enumerate().collect(), None, wd)
            }
        };

        let metadata = AiConversationMetadata {
            exported_at: Utc::now(),
            scope,
            tab_title: tab.title.clone(),
            pane_title,
            working_directory,
            host: self.host_info.display.clone(),
        };

        let mut messages = Vec::new();
        let mut referenced_blocks = Vec::new();
        let mut seen_blocks: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();

        for (pane_id, pane) in panes {
            for message in &pane.ai_chat {
                let created_at = pane
                    .history
                    .iter()
                    .filter_map(|block| block.ended_at.or(block.started_at))
                    .last()
                    .unwrap_or_else(Utc::now);
                messages.push(AiConversationMessage {
                    role: message.role,
                    content: message.content.clone(),
                    created_at,
                    sources: message.sources.clone(),
                    pane_title: Some(pane.title.clone()),
                });

                for citation in &message.sources {
                    if let Some(block_index) = citation.block_index {
                        let key = (pane_id, block_index);
                        if seen_blocks.insert(key) {
                            if let Some(block) = pane.history.get(block_index) {
                                referenced_blocks.push(AiReferencedBlock {
                                    pane_id,
                                    pane_title: pane.title.clone(),
                                    block_index,
                                    command: block.command.clone(),
                                    output: block.output.clone(),
                                    exit_code: block.exit_code,
                                    duration_ms: block.duration_ms,
                                    started_at: block.started_at,
                                    ended_at: block.ended_at,
                                    cwd: block.cwd.as_ref().map(|path| path.display().to_string()),
                                    git_branch: block.git_branch.clone(),
                                    git_status: block.git_status.clone(),
                                    host: block.host.clone(),
                                    is_remote: block.is_remote,
                                    tags: block.tags.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }

        Some(AiConversationExport {
            metadata,
            messages,
            referenced_blocks,
        })
    }

    fn handle_ai_export(&mut self, export: AiConversationExport, scope: AiConversationExportScope, format: ExportFormat) -> Command<Message> {
        if let Ok(result) = format_ai_conversation_export(&export, format) {
            if let Ok(path) = write_ai_export_file(std::path::Path::new("exports"), scope, format, &result.content) {
                let mut toast_message = format!("AI export saved: {}", path.display());
                if self.app_config.ai_share_link_enabled {
                    let link = format!("file://{}", path.display());
                    toast_message.push_str(&format!(" • Link copied: {}", link));
                    self.export_toast = Some(ExportToast {
                        message: toast_message,
                        expires_at: Utc::now() + chrono::Duration::seconds(6),
                    });
                    return clipboard::write(link);
                }
                self.export_toast = Some(ExportToast {
                    message: toast_message,
                    expires_at: Utc::now() + chrono::Duration::seconds(6),
                });
            }
        }
        Command::none()
    }


    fn update_history_matches(&mut self) {
        let query = self.history_search_query.to_lowercase();
        let mut matches = Vec::new();
        if let Some(tab) = self.layout.get(self.active_tab) {
            for block in tab.panes.get(tab.active_pane).map(|p| &p.history).into_iter().flatten().rev() {
                if query.is_empty() || block.command.to_lowercase().contains(&query) {
                    matches.push(block.command.clone());
                }
                if matches.len() >= 10 {
                    break;
                }
            }
        }
        self.history_matches = matches;
        self.history_selected = 0;
    }

    fn apply_history_selection(&mut self) {
        if let Some(selected) = self.history_matches.get(self.history_selected).cloned() {
            if let Some(tab) = self.layout.get_mut(self.active_tab) {
                if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                    pane.current_command = selected;
                }
            }
        }
        self.history_search_active = false;
        self.history_search_query.clear();
        self.history_matches.clear();
        self.history_selected = 0;
    }
    fn create_new_tab(&mut self) {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        let home = std::env::var("HOME").ok();
        let pane = match Pane::new(&shell, home) {
            Ok(pane) => pane,
            Err(err) => {
                error!("Failed to create pane for new tab: {}", err);
                return;
            }
        };
        let title = format!("Tab {}", self.layout.len() + 1);
        let tab = Tab { root: LayoutNode::Leaf { pane_id: 0 }, panes: vec![pane], active_pane: 0, title };
        self.layout.push(tab);
        self.active_tab = self.layout.len().saturating_sub(1);
        self.render_cache.lock().unwrap().clear();
        self.row_hashes.lock().unwrap().clear();
    }

    fn close_tab_at(&mut self, index: usize) {
        if self.layout.len() <= 1 {
            warn!("Cannot close the last tab");
            return;
        }
        if let Some(tab) = self.layout.get(index) {
            let has_running = tab.panes.iter().any(|pane| {
                pane.current_block.as_ref().map(|b| b.exit_code.is_none()).unwrap_or(false)
            });
            if has_running {
                warn!("Tab has running commands; close aborted");
                return;
            }
        }

        if index < self.layout.len() {
            self.layout.remove(index);
            if self.active_tab >= self.layout.len() {
                self.active_tab = self.layout.len().saturating_sub(1);
            } else if self.active_tab > index {
                self.active_tab = self.active_tab.saturating_sub(1);
            }
            self.render_cache.lock().unwrap().clear();
            self.row_hashes.lock().unwrap().clear();
        }
    }

    fn begin_rename_tab(&mut self, index: usize) {
        if let Some(tab) = self.layout.get(index) {
            self.renaming_tab = Some(index);
            self.rename_buffer = tab.title.clone();
        }
    }

    fn commit_rename_tab(&mut self) {
        if let Some(index) = self.renaming_tab.take() {
            if let Some(tab) = self.layout.get_mut(index) {
                let trimmed = self.rename_buffer.trim();
                if !trimmed.is_empty() {
                    tab.title = trimmed.to_string();
                }
            }
        }
        self.rename_buffer.clear();
    }
    fn remove_pane_from_layout(node: LayoutNode, pane_id: usize) -> Option<LayoutNode> {
        match node {
            LayoutNode::Leaf { pane_id: id } => {
                if id == pane_id {
                    None
                } else {
                    Some(LayoutNode::Leaf { pane_id: id })
                }
            }
            LayoutNode::Split { axis, ratio, left, right } => {
                let left = Self::remove_pane_from_layout(*left, pane_id);
                let right = Self::remove_pane_from_layout(*right, pane_id);
                match (left, right) {
                    (None, None) => None,
                    (Some(node), None) | (None, Some(node)) => Some(node),
                    (Some(left), Some(right)) => Some(LayoutNode::Split {
                        axis,
                        ratio,
                        left: Box::new(left),
                        right: Box::new(right),
                    }),
                }
            }
        }
    }

    fn reindex_layout(node: &mut LayoutNode, removed_id: usize) {
        match node {
            LayoutNode::Leaf { pane_id } => {
                if *pane_id > removed_id {
                    *pane_id -= 1;
                }
            }
            LayoutNode::Split { left, right, .. } => {
                Self::reindex_layout(left, removed_id);
                Self::reindex_layout(right, removed_id);
            }
        }
    }

    fn close_active_pane(&mut self) {
        if let Some(tab) = self.layout.get_mut(self.active_tab) {
            if tab.panes.len() <= 1 {
                warn!("Cannot close the last pane in a tab");
                return;
            }

            let active_id = tab.active_pane;
            if let Some(pane) = tab.panes.get(active_id) {
                if pane.current_block.as_ref().map(|b| b.exit_code.is_none()).unwrap_or(false) {
                    warn!("Pane has a running command; close aborted");
                    return;
                }
            }

            tab.panes.remove(active_id);
            let old_root = std::mem::replace(&mut tab.root, LayoutNode::Leaf { pane_id: 0 });
            if let Some(new_root) = Self::remove_pane_from_layout(old_root, active_id) {
                tab.root = new_root;
            } else if !tab.panes.is_empty() {
                tab.root = LayoutNode::Leaf { pane_id: 0 };
            }
            Self::reindex_layout(&mut tab.root, active_id);

            if tab.panes.is_empty() {
                tab.active_pane = 0;
            } else if active_id > 0 {
                tab.active_pane = active_id - 1;
            } else {
                tab.active_pane = 0;
            }
        }
    }
    fn split_layout_node(node: &mut LayoutNode, target_pane_id: usize, new_pane_id: usize, axis: Axis) -> bool {
        match node {
            LayoutNode::Leaf { pane_id } => {
                if *pane_id == target_pane_id {
                    *node = LayoutNode::Split {
                        axis,
                        ratio: 0.5,
                        left: Box::new(LayoutNode::Leaf { pane_id: target_pane_id }),
                        right: Box::new(LayoutNode::Leaf { pane_id: new_pane_id }),
                    };
                    true
                } else {
                    false
                }
            }
            LayoutNode::Split { left, right, .. } => {
                if Self::split_layout_node(left, target_pane_id, new_pane_id, axis) {
                    return true;
                }
                Self::split_layout_node(right, target_pane_id, new_pane_id, axis)
            }
        }
    }

    fn contains_pane(node: &LayoutNode, pane_id: usize) -> bool {
        match node {
            LayoutNode::Leaf { pane_id: id } => *id == pane_id,
            LayoutNode::Split { left, right, .. } => {
                Self::contains_pane(left, pane_id) || Self::contains_pane(right, pane_id)
            }
        }
    }

    fn find_split_path_for_pane(node: &LayoutNode, pane_id: usize, axis: Axis, path: &mut Vec<SplitDirection>) -> Option<Vec<SplitDirection>> {
        match node {
            LayoutNode::Split { axis: node_axis, left, right, .. } => {
                if *node_axis == axis && (Self::contains_pane(left, pane_id) || Self::contains_pane(right, pane_id)) {
                    return Some(path.clone());
                }
                if Self::contains_pane(left, pane_id) {
                    path.push(SplitDirection::Left);
                    let found = Self::find_split_path_for_pane(left, pane_id, axis, path);
                    path.pop();
                    if found.is_some() {
                        return found;
                    }
                }
                if Self::contains_pane(right, pane_id) {
                    path.push(SplitDirection::Right);
                    let found = Self::find_split_path_for_pane(right, pane_id, axis, path);
                    path.pop();
                    return found;
                }
                None
            }
            LayoutNode::Leaf { .. } => None,
        }
    }

    fn get_split_node_mut<'a>(node: &'a mut LayoutNode, path: &[SplitDirection]) -> Option<&'a mut LayoutNode> {
        if path.is_empty() {
            return Some(node);
        }
        match node {
            LayoutNode::Split { left, right, .. } => match path[0] {
                SplitDirection::Left => Self::get_split_node_mut(left, &path[1..]),
                SplitDirection::Right => Self::get_split_node_mut(right, &path[1..]),
            },
            LayoutNode::Leaf { .. } => None,
        }
    }

    fn split_active_pane(&mut self, axis: Axis) {
        if let Some(tab) = self.layout.get_mut(self.active_tab) {
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
            let working_directory = tab
                .panes
                .get(tab.active_pane)
                .map(|pane| pane.working_directory.clone());
            let new_pane = match Pane::new(&shell, working_directory) {
                Ok(pane) => pane,
                Err(err) => {
                    error!("Failed to create pane: {}", err);
                    return;
                }
            };
            let new_pane_id = tab.panes.len();
            tab.panes.push(new_pane);
            let replaced = Self::split_layout_node(&mut tab.root, tab.active_pane, new_pane_id, axis);
            if replaced {
                tab.active_pane = new_pane_id;
            } else {
                error!("Failed to split pane: active pane not found in layout");
            }
        }
    }

    fn find_split_hit(node: &LayoutNode, rect: Rectangle, position: Point, threshold: f32, path: &mut Vec<SplitDirection>) -> Option<SplitResizeState> {
        match node {
            LayoutNode::Split { axis, ratio, left, right } => {
                let (left_rect, right_rect, divider_rect) = match axis {
                    Axis::Horizontal => {
                        let divider_x = rect.x + rect.width * ratio;
                        let left_rect = Rectangle { x: rect.x, y: rect.y, width: rect.width * ratio, height: rect.height };
                        let right_rect = Rectangle { x: divider_x, y: rect.y, width: rect.width - rect.width * ratio, height: rect.height };
                        let divider_rect = Rectangle { x: divider_x - threshold, y: rect.y, width: threshold * 2.0, height: rect.height };
                        (left_rect, right_rect, divider_rect)
                    }
                    Axis::Vertical => {
                        let divider_y = rect.y + rect.height * ratio;
                        let left_rect = Rectangle { x: rect.x, y: rect.y, width: rect.width, height: rect.height * ratio };
                        let right_rect = Rectangle { x: rect.x, y: divider_y, width: rect.width, height: rect.height - rect.height * ratio };
                        let divider_rect = Rectangle { x: rect.x, y: divider_y - threshold, width: rect.width, height: threshold * 2.0 };
                        (left_rect, right_rect, divider_rect)
                    }
                };

                if divider_rect.contains(position) {
                    return Some(SplitResizeState { path: path.clone(), rect });
                }

                if left_rect.contains(position) {
                    path.push(SplitDirection::Left);
                    let found = Self::find_split_hit(left, left_rect, position, threshold, path);
                    path.pop();
                    if found.is_some() {
                        return found;
                    }
                }

                if right_rect.contains(position) {
                    path.push(SplitDirection::Right);
                    let found = Self::find_split_hit(right, right_rect, position, threshold, path);
                    path.pop();
                    return found;
                }

                None
            }
            LayoutNode::Leaf { .. } => None,
        }
    }

    fn update_split_ratio(node: &mut LayoutNode, state: &SplitResizeState, position: Point) {
        if let Some(target) = Self::get_split_node_mut(node, &state.path) {
            if let LayoutNode::Split { axis, ratio, .. } = target {
                let new_ratio = match axis {
                    Axis::Horizontal => ((position.x - state.rect.x) / state.rect.width).clamp(0.1, 0.9),
                    Axis::Vertical => ((position.y - state.rect.y) / state.rect.height).clamp(0.1, 0.9),
                };
                *ratio = new_ratio;
            }
        }
    }

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
                        title: pane.title.clone(),
                        scroll_offset: pane.scroll_offset,
                    }
                }).collect(),
                active_pane: tab.active_pane,
                title: tab.title.clone(),
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

    fn export_theme(&self) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self.theme_config)?;
        std::fs::write("theme.json", json)?;
        Ok(())
    }

    fn import_theme(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let json = std::fs::read_to_string("theme.json")?;
        self.theme_config = serde_json::from_str(&json)?;
        Ok(())
    }

    fn plan_limits(plan: PlanTier) -> PlanLimits {
        match plan {
            PlanTier::Free => PlanLimits {
                daily_messages: 50,
                daily_chars: 20_000,
                max_context_chars: 6_000,
                allow_streaming: false,
            },
            PlanTier::Pro => PlanLimits {
                daily_messages: 500,
                daily_chars: 200_000,
                max_context_chars: 24_000,
                allow_streaming: true,
            },
            PlanTier::Team => PlanLimits {
                daily_messages: 2_000,
                daily_chars: 1_000_000,
                max_context_chars: 64_000,
                allow_streaming: true,
            },
        }
    }

    fn usage_ledger_path() -> &'static str {
        "usage_ledger.json"
    }

    fn load_usage_ledger() -> UsageLedger {
        let path = std::path::Path::new(Self::usage_ledger_path());
        if let Ok(contents) = std::fs::read_to_string(path) {
            serde_json::from_str::<UsageLedger>(&contents).unwrap_or_else(|_| UsageLedger {
                entries: Vec::new(),
                last_reset: Utc::now(),
            })
        } else {
            UsageLedger { entries: Vec::new(), last_reset: Utc::now() }
        }
    }

    fn save_usage_ledger(&self) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self.usage_ledger)?;
        std::fs::write(Self::usage_ledger_path(), json)?;
        Ok(())
    }


    fn reset_usage_if_needed(&mut self) {
        let now = Utc::now();
        if now.date_naive() != self.usage_ledger.last_reset.date_naive() {
            self.usage_ledger.entries.clear();
            self.usage_ledger.last_reset = now;
        }
    }

    fn compute_usage_snapshot(&self, limits: &PlanLimits) -> UsageSnapshot {
        let used_messages = self.usage_ledger.entries.len() as u32;
        let used_chars = self
            .usage_ledger
            .entries
            .iter()
            .map(|entry| entry.request_chars as u32 + entry.response_chars as u32)
            .sum::<u32>();
        UsageSnapshot {
            used_messages,
            used_chars,
            remaining_messages: limits.daily_messages.saturating_sub(used_messages),
            remaining_chars: limits.daily_chars.saturating_sub(used_chars),
        }
    }

    fn can_send_ai_request(&self) -> bool {
        self.usage_snapshot.remaining_messages > 0 && self.usage_snapshot.remaining_chars > 0 && self.ai_settings.enabled
    }

    fn build_ai_prompt(&self, action: &str, user_text: &str, context: &str) -> AiRequest {
        let system = match action {
            "explain_error" => "Explain the error and likely cause in concise terms.",
            "suggest_fix" => "Suggest a practical fix and a command to try.",
            "generate_command" => "Generate a command for the user's request.",
            "summarize_output" => "Summarize the command output briefly.",
            _ => "Respond concisely.",
        };
        let model = match self.ai_settings.provider.as_str() {
            "openai" => "gpt-4o-mini",
            "anthropic" => "claude-3-5-sonnet-20240620",
            "ollama" => "llama3.2:latest",
            _ => "gpt-4o-mini",
        };
        let user = if user_text.trim().is_empty() {
            format!("{}", context)
        } else {
            format!("{}\n\nUser request:\n{}", context, user_text)
        };
        AiRequest {
            provider: self.ai_settings.provider.clone(),
            model: model.to_string(),
            system: system.to_string(),
            user,
        }
    }


    fn build_ai_request(&self, prompt: &str, action: &str) -> AiRequest {
        self.build_ai_prompt(action, "", prompt)
    }

    fn execute_palette_action(&mut self, action: PaletteAction) {
        match action {
            PaletteAction::SplitPaneHorizontal => {
                self.split_active_pane(Axis::Horizontal);
            }
            PaletteAction::SplitPaneVertical => {
                self.split_active_pane(Axis::Vertical);
            }
            PaletteAction::ClosePane => {
                self.close_active_pane();
            }
            PaletteAction::SwitchTab(index) => {
                if index < self.layout.len() {
                    self.active_tab = index;
                }
            }
            PaletteAction::RunPinnedCommand(index) => {
                if let Some(tab) = self.layout.get(self.active_tab) {
                    if let Some(pane) = tab.panes.get(tab.active_pane) {
                        if let Some(block) = pane.history.get(index) {
                            if block.pinned {
                                if let Ok(mut pty) = pane.pty.try_lock() {
                                    let cmd = format!("{}\r", block.command);
                                    if let Err(err) = pty.writer().write_all(cmd.as_bytes()) {
                                        error!("Failed to write to PTY: {}", err);
                                    }
                                    if let Err(err) = pty.writer().flush() {
                                        error!("Failed to flush PTY writer: {}", err);
                                    }
                                    info!("Run pinned command: {}", block.command);
                                }
                            }
                        }
                    }
                }
            }
            PaletteAction::ToggleAi => {
                self.ai_settings.enabled = !self.ai_settings.enabled;
            }
            PaletteAction::ToggleAiPanel => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                        pane.ai_panel_open = !pane.ai_panel_open;
                    }
                }
            }
            PaletteAction::SetAiContextScope(scope) => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                        pane.ai_context_scope = scope;
                    }
                }
            }
            PaletteAction::ApplyAiTemplate(template_id) => {
                let pane_id = self.layout
                    .get(self.active_tab)
                    .map(|tab| tab.active_pane)
                    .unwrap_or(0);
                self.apply_ai_template(pane_id, template_id, true);
            }
            PaletteAction::ExportTheme => {
                if let Err(e) = self.export_theme() {
                    error!("Failed to export theme: {}", e);
                }
            }
            PaletteAction::ImportTheme => {
                if let Err(e) = self.import_theme() {
                    error!("Failed to import theme: {}", e);
                }
            }
            PaletteAction::OpenBilling => {
                self.show_billing = true;
            }
        }
    }

    fn render_command_palette(&self) -> Element<Message> {
        let actions = self.get_available_actions();
        let filtered_actions: Vec<_> = actions.iter()
            .filter(|(name, _)| name.to_lowercase().contains(&self.palette_query.to_lowercase()))
            .collect();

        let mut column = Column::new().spacing(5).padding(20);

        // Search input
        let search_input = TextInput::new("Search commands...", &self.palette_query)
            .on_input(Message::UpdatePaletteQuery)
            .padding(10);
        column = column.push(search_input);

        // Action list
        for (index, (name, action)) in filtered_actions.iter().enumerate() {
            let is_selected = index == self.palette_selected;
            let mut text = iced::widget::Text::new(*name);
            if is_selected {
                text = text.style(Color::from_rgb(0.4, 0.7, 0.9));
            }
            let button = iced::widget::Button::new(text)
                .on_press(Message::ExecutePaletteAction((*action).clone()))
                .padding(5);
            column = column.push(button);
        }

        container(column)
            .center_x()
            .center_y()
            .width(Length::Fixed(400.0))
            .height(Length::Fixed(300.0))
            .into()
    }

    fn render_billing(&self) -> Element<Message> {
        let limits = Self::plan_limits(self.billing_profile.plan);
        let plan_label = match self.billing_profile.plan {
            PlanTier::Free => "Free",
            PlanTier::Pro => "Pro",
            PlanTier::Team => "Team",
        };
        let usage = Column::new()
            .spacing(6)
            .push(iced::widget::Text::new(format!(
                "Usage today: {} / {} messages",
                self.usage_snapshot.used_messages, limits.daily_messages
            )))
            .push(iced::widget::Text::new(format!(
                "Usage today: {} / {} chars",
                self.usage_snapshot.used_chars, limits.daily_chars
            )))
            .push(iced::widget::Text::new(format!(
                "Streaming: {}",
                if limits.allow_streaming { "enabled" } else { "disabled" }
            )));
        let plan_buttons = Row::new()
            .spacing(8)
            .push(iced::widget::Button::new(iced::widget::Text::new("Free")).on_press(Message::BillingPlanSelected(PlanTier::Free)))
            .push(iced::widget::Button::new(iced::widget::Text::new("Pro")).on_press(Message::BillingPlanSelected(PlanTier::Pro)))
            .push(iced::widget::Button::new(iced::widget::Text::new("Team")).on_press(Message::BillingPlanSelected(PlanTier::Team)));
        let actions = Row::new()
            .spacing(8)
            .push(iced::widget::Button::new(iced::widget::Text::new("Upgrade")).on_press(Message::BillingUpgradeRequested))
            .push(iced::widget::Button::new(iced::widget::Text::new("Sync billing")).on_press(Message::BillingSyncRequested))
            .push(iced::widget::Button::new(iced::widget::Text::new("Close")).on_press(Message::CloseBilling));
        let content = Column::new()
            .spacing(10)
            .push(iced::widget::Text::new("Billing").size(18.0))
            .push(iced::widget::Text::new(format!("Current plan: {}", plan_label)))
            .push(usage)
            .push(iced::widget::Text::new("Select plan:"))
            .push(plan_buttons)
            .push(actions)
            .padding(20);
        container(content)
            .center_x()
            .center_y()
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme: &Theme| container::Appearance {
                background: Some(Background::Color(Color::from_rgb(0.08, 0.09, 0.11))),
                border: Border {
                    radius: 8.0.into(),
                    width: 1.0,
                    color: Color::from_rgb(0.2, 0.2, 0.2),
                },
                ..Default::default()
            })
            .into()
    }

    fn render_ai_onboarding(&self) -> Element<Message> {
        let title = iced::widget::Text::new("Welcome to AI Assistant")
            .size(18.0)
            .style(Color::from_rgb(0.95, 0.95, 0.95));
        let body = Column::new()
            .spacing(8)
            .push(iced::widget::Text::new("Get better answers by adding context and choosing a template:").size(12.0))
            .push(iced::widget::Text::new("• Select context scope (current command, last blocks, selection). ").size(12.0))
            .push(iced::widget::Text::new("• Use templates to structure your request quickly. ").size(12.0))
            .push(iced::widget::Text::new("• Review redaction before sending sensitive data. ").size(12.0));
        let privacy = iced::widget::Text::new("Privacy: prompts include your selected context only. Redaction removes secrets unless you override.")
            .size(11.0)
            .style(Color::from_rgb(0.7, 0.7, 0.7));
        let buttons = Row::new()
            .spacing(8)
            .push(iced::widget::Button::new(iced::widget::Text::new("Get started").size(12.0)).on_press(Message::AiOnboardingContinue))
            .push(iced::widget::Button::new(iced::widget::Text::new("Skip").size(12.0)).on_press(Message::AiOnboardingSkip));
        let modal = Column::new()
            .spacing(12)
            .push(title)
            .push(body)
            .push(privacy)
            .push(buttons)
            .padding(20);
        container(modal)
            .center_x()
            .center_y()
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme: &Theme| container::Appearance {
                background: Some(Background::Color(Color::from_rgb(0.08, 0.09, 0.11))),
                border: Border {
                    radius: 8.0.into(),
                    width: 1.0,
                    color: Color::from_rgb(0.2, 0.2, 0.2),
                },
                ..Default::default()
            })
            .into()
    }

    fn get_available_actions(&self) -> Vec<(&str, PaletteAction)> {
        let mut actions = vec![
            ("Split Pane Horizontal", PaletteAction::SplitPaneHorizontal),
            ("Split Pane Vertical", PaletteAction::SplitPaneVertical),
            ("Close Pane", PaletteAction::ClosePane),
            ("Toggle AI", PaletteAction::ToggleAi),
            ("Toggle AI Panel", PaletteAction::ToggleAiPanel),
            ("Open Billing", PaletteAction::OpenBilling),
            ("AI Template: Explain error", PaletteAction::ApplyAiTemplate(AiPromptTemplateId::ExplainError)),
            ("AI Template: Debug command", PaletteAction::ApplyAiTemplate(AiPromptTemplateId::DebugCommand)),
            ("AI Template: Summarize output", PaletteAction::ApplyAiTemplate(AiPromptTemplateId::SummarizeOutput)),
            ("AI Template: Generate command", PaletteAction::ApplyAiTemplate(AiPromptTemplateId::GenerateCommand)),
            ("AI Template: Refactor snippet", PaletteAction::ApplyAiTemplate(AiPromptTemplateId::RefactorSnippet)),
            ("AI Context: Current", PaletteAction::SetAiContextScope(AiContextScope::CurrentCommand)),
            ("AI Context: Last N", PaletteAction::SetAiContextScope(AiContextScope::LastNBlocks)),
            ("AI Context: Selected", PaletteAction::SetAiContextScope(AiContextScope::SelectedBlocks)),
            ("AI Context: All", PaletteAction::SetAiContextScope(AiContextScope::EntireSession)),
            ("Export Theme", PaletteAction::ExportTheme),
            ("Import Theme", PaletteAction::ImportTheme),
        ];

        // Add switch tab actions
        for i in 0..self.layout.len() {
            actions.push((Box::leak(format!("Switch to Tab {}", i + 1).into_boxed_str()), PaletteAction::SwitchTab(i)));
        }

        // Add pinned commands
        if let Some(tab) = self.layout.get(self.active_tab) {
            if let Some(pane) = tab.panes.get(tab.active_pane) {
                for (index, block) in pane.history.iter().enumerate() {
                    if block.pinned {
                        actions.push((Box::leak(format!("Run: {}", block.command).into_boxed_str()), PaletteAction::RunPinnedCommand(index)));
                    }
                }
            }
        }

        actions
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
                    pane.title = saved_pane.title;
                    pane.scroll_offset = saved_pane.scroll_offset;
                    panes.push(pane);
                }
                let tab = Tab { root: saved_tab.root, panes, active_pane: saved_tab.active_pane, title: saved_tab.title };
                tabs.push(tab);
            }
            (tabs, saved_layout.active_tab)
        } else {
            // Default: single pane
            let pane = Pane::new(&shell, None).unwrap();
            let root = LayoutNode::Leaf { pane_id: 0 };
            let tab = Tab { root, panes: vec![pane], active_pane: 0, title: "Tab 1".to_string() };
            (vec![tab], 0)
        };
        let renderer = TerminalRenderer::new();
        let app_config = Self::load_app_config();
        let ai_onboarding_open = !app_config.ai_onboarding_seen;
        let usage_ledger = Self::load_usage_ledger();
        let billing_profile = BillingProfile {
            plan: app_config.plan_tier,
            customer_id: None,
            subscription_id: None,
            renewal_at: None,
        };
        let plan_limits = Self::plan_limits(billing_profile.plan);
        let usage_snapshot = {
            let mut ledger = usage_ledger.clone();
            if Utc::now().date_naive() != ledger.last_reset.date_naive() {
                ledger.entries.clear();
                ledger.last_reset = Utc::now();
            }
            UsageSnapshot {
                used_messages: ledger.entries.len() as u32,
                used_chars: ledger
                    .entries
                    .iter()
                    .map(|entry| entry.request_chars as u32 + entry.response_chars as u32)
                    .sum::<u32>(),
                remaining_messages: plan_limits.daily_messages.saturating_sub(ledger.entries.len() as u32),
                remaining_chars: plan_limits.daily_chars.saturating_sub(
                    ledger
                        .entries
                        .iter()
                        .map(|entry| entry.request_chars as u32 + entry.response_chars as u32)
                        .sum::<u32>(),
                ),
            }
        };
        let ai_settings = AiSettings {
            enabled: true,
            send_current_command: true,
            send_last_n_blocks: 5,
            send_repo_context: false,
            provider: "openai".to_string(),  // Changed from ollama to openai
            api_key: None,
            redact_secrets: true,
            allow_sensitive: false,
        };
        let theme_config = preset_theme("one_dark");
        (
            Tant { layout, active_tab, renderer, search_query: String::new(), search_success_only: false, search_failure_only: false, search_pinned_only: false, search_input_id: text_input::Id::unique(), ai_settings, ai_response: None, app_config, ai_onboarding_open, show_command_palette: false, palette_query: String::new(), palette_selected: 0, render_cache: Arc::new(Mutex::new(HashMap::new())), row_hashes: Arc::new(Mutex::new(HashMap::new())), theme_config, host_info: resolve_host_info(), window_size: Size::new(1024.0, 768.0), resize_state: None, last_cursor_pos: Point { x: 0.0, y: 0.0 }, renaming_tab: None, rename_buffer: String::new(), history_search_active: false, history_search_query: String::new(), history_matches: Vec::new(), history_selected: 0, export_toast: None, usage_ledger, billing_profile, usage_snapshot, show_billing: false },
            window::gain_focus(window::Id::MAIN)
        )
    }

    fn title(&self) -> String {
        "Tant Terminal".to_string()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Tick => {
                self.reset_usage_if_needed();
                let limits = Self::plan_limits(self.billing_profile.plan);
                self.usage_snapshot = self.compute_usage_snapshot(&limits);
                self.ai_settings.send_last_n_blocks = self.ai_settings.send_last_n_blocks.min(limits.max_context_chars / 1000).max(1);
                for tab in &mut self.layout {
                    for pane in &mut tab.panes {
                        if pane.ai_streaming {
                            if !limits.allow_streaming {
                                pane.ai_streaming = false;
                                pane.ai_stream_remaining.clear();
                                pane.ai_stream_target = None;
                            }
                            if let Some(target) = pane.ai_stream_target {
                                if let Some(message) = pane.ai_chat.get_mut(target) {
                                    if !pane.ai_stream_remaining.is_empty() {
                                        let chunk_size = 6;
                                        let take = pane.ai_stream_remaining.char_indices().nth(chunk_size).map(|(idx, _)| idx).unwrap_or(pane.ai_stream_remaining.len());
                                        let chunk: String = pane.ai_stream_remaining.drain(..take).collect();
                                        message.content.push_str(&chunk);
                                    }
                                    if pane.ai_stream_remaining.is_empty() && !pane.ai_pending {
                                        pane.ai_streaming = false;
                                        pane.ai_stream_target = None;
                                    }
                                } else {
                                    pane.ai_streaming = false;
                                    pane.ai_stream_target = None;
                                }
                            }
                        }
                    }
                }
                if let Some(toast) = &self.export_toast {
                    if toast.expires_at <= Utc::now() {
                        self.export_toast = None;
                    }
                }
                let mut has_new_data = false;
                for tab in &mut self.layout {
                    for pane in &mut tab.panes {
                        // Receive data from the async reader
                        while let Ok(data) = pane.data_receiver.try_recv() {
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
                                    debug!("[Block Detection] Prompt shown");
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
                                        selected: false,
                                        output: String::new(),
                                        git_branch: None,
                                        git_status: None,
                                        host: self.host_info.display.clone(),
                                        is_remote: self.host_info.is_remote,
                                        collapsed: false,
                                    });
                                    debug!("[Block Detection] Command started - new block created");
                                }
                                ParserEvent::Command(cmd) => {
                                    if let Some(ref mut block) = pane.current_block {
                                        block.command = cmd;
                                    }
                                }
                                ParserEvent::Directory(dir) => {
                                    if let Some(ref mut block) = pane.current_block {
                                        block.cwd = Some(std::path::PathBuf::from(&dir));
                                    }
                                    pane.working_directory = dir;
                                }
                                ParserEvent::GitInfo { branch, status } => {
                                    if let Some(ref mut block) = pane.current_block {
                                        block.git_branch = Some(branch);
                                        block.git_status = status;
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
                                        debug!("[Block Detection] Command ended with status {} - block saved", status);
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
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                        if let Ok(mut pty) = pane.pty.try_lock() {
                            let bytes = Self::key_to_bytes(&key, &modifiers);
                            if !bytes.is_empty() {
                                if let Err(err) = pty.writer().write_all(&bytes) {
                                    error!("Failed to write to PTY: {}", err);
                                }
                                if let Err(err) = pty.writer().flush() {
                                    error!("Failed to flush PTY writer: {}", err);
                                }
                            }
                        }
                    }
                }
                Command::none()
            }
            Message::TextInput(text) => {
                if self.history_search_active {
                    let next = format!("{}{}", self.history_search_query, text);
                    self.history_search_query = next;
                    self.update_history_matches();
                    return Command::none();
                }
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                        if let Ok(mut pty) = pane.pty.try_lock() {
                            if let Err(err) = pty.writer().write_all(text.as_bytes()) {
                                error!("Failed to write to PTY: {}", err);
                            }
                            if let Err(err) = pty.writer().flush() {
                                error!("Failed to flush PTY writer: {}", err);
                            }
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
                            if let Err(err) = pty.writer().write_all(b"\x1b[200~") {
                                error!("Failed to write to PTY: {}", err);
                            }
                            if let Err(err) = pty.writer().write_all(text.as_bytes()) {
                                error!("Failed to write to PTY: {}", err);
                            }
                            if let Err(err) = pty.writer().write_all(b"\x1b[201~") {
                                error!("Failed to write to PTY: {}", err);
                            }
                            if let Err(err) = pty.writer().flush() {
                                error!("Failed to flush PTY writer: {}", err);
                            }
                        }
                    }
                }
                Command::none()
            }
            Message::Resize(width, height) => {
                self.window_size = Size::new(width as f32, height as f32);
                let (cell_w, cell_h) = self.renderer.cell_size(&self.theme_config);
                let cols = (width as f32 / cell_w) as u16;
                let rows = (height as f32 / cell_h) as u16;
                let pixel_width = width as u16;
                let pixel_height = height as u16;
                for tab in &mut self.layout {
                    for pane in &mut tab.panes {
                        pane.parser.resize(rows, cols);
                        if let Ok(mut pty) = pane.pty.try_lock() {
                            if let Err(err) = pty.resize(rows, cols, pixel_width, pixel_height) {
                                error!("Failed to resize PTY: {}", err);
                            }
                        }
                    }
                }
                // Clear render caches on resize
                self.render_cache.lock().unwrap().clear();
                self.row_hashes.lock().unwrap().clear();
                Command::none()
            }
            Message::KeyboardEvent(key, modifiers, text) => {
                let is_cmd = modifiers.command();
                let is_ctrl = modifiers.control();
                let is_shift = modifiers.shift();

                if (is_cmd || is_ctrl) && matches!(key, Key::Character(ref c) if c == "i") {
                    return self.update(Message::ToggleAiPanel);
                }

                if is_ctrl && matches!(key, Key::Character(ref c) if c == "r") {
                    return self.update(Message::StartHistorySearch);
                }

                if self.history_search_active {
                    if matches!(key, Key::Named(iced::keyboard::key::Named::Escape)) {
                        return self.update(Message::CancelHistorySearch);
                    }
                    if matches!(key, Key::Named(iced::keyboard::key::Named::ArrowDown)) {
                        return self.update(Message::NextHistoryMatch);
                    }
                    if matches!(key, Key::Named(iced::keyboard::key::Named::ArrowUp)) {
                        return self.update(Message::PrevHistoryMatch);
                    }
                    if matches!(key, Key::Named(iced::keyboard::key::Named::Enter)) {
                        return self.update(Message::ApplyHistoryMatch);
                    }
                    if let Some(txt) = text.clone() {
                        if !txt.is_empty() {
                            let next = format!("{}{}", self.history_search_query, txt);
                            return self.update(Message::UpdateHistorySearch(next));
                        }
                    }
                    if matches!(key, Key::Named(iced::keyboard::key::Named::Backspace)) {
                        if !self.history_search_query.is_empty() {
                            let mut next = self.history_search_query.clone();
                            next.pop();
                            return self.update(Message::UpdateHistorySearch(next));
                        }
                    }
                }

                if matches!(key, Key::Character(ref c) if c == "t") {
                    if is_cmd || is_ctrl {
                        return self.update(Message::NewTab);
                    }
                }

                if matches!(key, Key::Character(ref c) if c == "q") {
                    if is_ctrl && is_shift {
                        return self.update(Message::CloseTab);
                    }
                }

                if matches!(key, Key::Character(ref c) if c == "d") {
                    if is_cmd && !is_shift {
                        return self.update(Message::SplitPaneHorizontal);
                    }
                    if is_cmd && is_shift {
                        return self.update(Message::SplitPaneVertical);
                    }
                    if is_ctrl && is_shift {
                        return self.update(Message::SplitPaneHorizontal);
                    }
                    if is_ctrl && !is_shift {
                        return self.update(Message::SplitPaneVertical);
                    }
                }

                if matches!(key, Key::Character(ref c) if c == "w") {
                    if is_cmd {
                        if let Some(tab) = self.layout.get(self.active_tab) {
                            if tab.panes.len() > 1 {
                                return self.update(Message::ClosePane);
                            }
                        }
                        return self.update(Message::CloseTab);
                    }
                    if is_ctrl && is_shift {
                        return self.update(Message::ClosePane);
                    }
                }

                if matches!(key, Key::Character(ref c) if c == "r") {
                    if is_cmd && is_shift {
                        let index = self.active_tab;
                        return self.update(Message::BeginRenameTab(index));
                    }
                }

                if is_cmd {
                    if let Key::Character(ref c) = key {
                        if let Ok(num) = c.parse::<usize>() {
                            if num >= 1 && num <= 9 {
                                let index = num - 1;
                                return self.update(Message::SelectTab(index));
                            }
                        }
                    }
                }

                if is_cmd && matches!(key, Key::Character(ref c) if c == "[") {
                    return self.update(Message::PrevTab);
                }
                if is_cmd && matches!(key, Key::Character(ref c) if c == "]") {
                    return self.update(Message::NextTab);
                }

                if matches!(key, Key::Named(iced::keyboard::key::Named::Escape)) {
                    if self.renaming_tab.is_some() {
                        return self.update(Message::CancelRenameTab);
                    }
                }

                if is_cmd && matches!(key, Key::Character(ref c) if c == "[") {
                    return self.update(Message::SwitchPane(-1));
                }
                if is_cmd && matches!(key, Key::Character(ref c) if c == "]") {
                    return self.update(Message::SwitchPane(1));
                }

                if is_ctrl && modifiers.alt() {
                    match key {
                        Key::Named(iced::keyboard::key::Named::ArrowLeft) => return self.update(Message::SwitchPane(-1)),
                        Key::Named(iced::keyboard::key::Named::ArrowRight) => return self.update(Message::SwitchPane(1)),
                        Key::Named(iced::keyboard::key::Named::ArrowUp) => return self.update(Message::SwitchPane(-1)),
                        Key::Named(iced::keyboard::key::Named::ArrowDown) => return self.update(Message::SwitchPane(1)),
                        _ => {}
                    }
                }

                if is_cmd && is_ctrl {
                    match key {
                        Key::Named(iced::keyboard::key::Named::ArrowLeft) => return self.update(Message::AdjustSplitRatio(Axis::Horizontal, -0.05)),
                        Key::Named(iced::keyboard::key::Named::ArrowRight) => return self.update(Message::AdjustSplitRatio(Axis::Horizontal, 0.05)),
                        Key::Named(iced::keyboard::key::Named::ArrowUp) => return self.update(Message::AdjustSplitRatio(Axis::Vertical, -0.05)),
                        Key::Named(iced::keyboard::key::Named::ArrowDown) => return self.update(Message::AdjustSplitRatio(Axis::Vertical, 0.05)),
                        _ => {}
                    }
                }

                if modifiers.control() && matches!(key, iced::keyboard::Key::Character(ref c) if c == "k") {
                    return self.update(Message::OpenCommandPalette);
                }

                if (modifiers.command() || modifiers.control()) && matches!(key, iced::keyboard::Key::Character(ref c) if c == "f") {
                    return self.update(Message::FocusSearch);
                }

                if matches!(key, iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape)) {
                    return self.update(Message::ClearSearch);
                }

                if matches!(key, iced::keyboard::Key::Named(iced::keyboard::key::Named::Space)) {
                    return self.update(Message::TextInput(" ".to_string()));
                }

                if modifiers.control() || modifiers.alt() || modifiers.logo() {
                    return self.update(Message::KeyPress(key.clone(), modifiers));
                }

                if matches!(key, iced::keyboard::Key::Named(_)) {
                    return self.update(Message::KeyPress(key.clone(), modifiers));
                }

                if let Some(txt) = text {
                    if !txt.is_empty() {
                        return self.update(Message::TextInput(txt));
                    }
                }

                if matches!(&key, iced::keyboard::Key::Character(c) if c == " ") {
                    return self.update(Message::TextInput(" ".to_string()));
                }

                self.update(Message::KeyPress(key.clone(), modifiers))
            }
            Message::StartHistorySearch => {
                self.history_search_active = true;
                self.history_search_query.clear();
                self.update_history_matches();
                Command::none()
            }
            Message::UpdateHistorySearch(query) => {
                self.history_search_query = query;
                self.update_history_matches();
                Command::none()
            }
            Message::NextHistoryMatch => {
                if !self.history_matches.is_empty() {
                    self.history_selected = (self.history_selected + 1) % self.history_matches.len();
                }
                Command::none()
            }
            Message::PrevHistoryMatch => {
                if !self.history_matches.is_empty() {
                    if self.history_selected == 0 {
                        self.history_selected = self.history_matches.len() - 1;
                    } else {
                        self.history_selected -= 1;
                    }
                }
                Command::none()
            }
            Message::ApplyHistoryMatch => {
                self.apply_history_selection();
                Command::none()
            }
            Message::CancelHistorySearch => {
                self.history_search_active = false;
                self.history_search_query.clear();
                self.history_matches.clear();
                self.history_selected = 0;
                Command::none()
            }
            Message::SplitPaneHorizontal => {
                self.split_active_pane(Axis::Horizontal);
                Command::none()
            }
            Message::SplitPaneVertical => {
                self.split_active_pane(Axis::Vertical);
                Command::none()
            }
            Message::ClosePane => {
                self.close_active_pane();
                Command::none()
            }
            Message::NewTab => {
                self.create_new_tab();
                Command::none()
            }
            Message::CloseTab => {
                self.close_tab_at(self.active_tab);
                Command::none()
            }
            Message::CloseTabAt(index) => {
                self.close_tab_at(index);
                Command::none()
            }
            Message::SelectTab(index) => {
                if index < self.layout.len() {
                    self.active_tab = index;
                }
                Command::none()
            }
            Message::PrevTab => {
                if !self.layout.is_empty() {
                    let len = self.layout.len() as isize;
                    self.active_tab = ((self.active_tab as isize - 1).rem_euclid(len)) as usize;
                }
                Command::none()
            }
            Message::NextTab => {
                if !self.layout.is_empty() {
                    let len = self.layout.len() as isize;
                    self.active_tab = ((self.active_tab as isize + 1).rem_euclid(len)) as usize;
                }
                Command::none()
            }
            Message::BeginRenameTab(index) => {
                self.begin_rename_tab(index);
                Command::none()
            }
            Message::RenameTabInput(value) => {
                self.rename_buffer = value;
                Command::none()
            }
            Message::CommitRenameTab => {
                self.commit_rename_tab();
                Command::none()
            }
            Message::CancelRenameTab => {
                self.renaming_tab = None;
                self.rename_buffer.clear();
                Command::none()
            }
            Message::SwitchPane(delta) => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if !tab.panes.is_empty() {
                        let len = tab.panes.len() as isize;
                        let next = (tab.active_pane as isize + delta).rem_euclid(len) as usize;
                        tab.active_pane = next;
                    }
                }
                Command::none()
            }
            Message::AdjustSplitRatio(axis, delta) => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    let mut path = Vec::new();
                    if let Some(target_path) = Self::find_split_path_for_pane(&tab.root, tab.active_pane, axis, &mut path) {
                        if let Some(target) = Self::get_split_node_mut(&mut tab.root, &target_path) {
                            if let LayoutNode::Split { ratio, .. } = target {
                                *ratio = (*ratio + delta).clamp(0.1, 0.9);
                            }
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
            Message::CopyOutput(index) => {
                if let Some(tab) = self.layout.get(self.active_tab) {
                    if let Some(pane) = tab.panes.get(tab.active_pane) {
                        if let Some(block) = pane.history.get(index) {
                            return clipboard::write(block.output.clone());
                        }
                    }
                }
                Command::none()
            }
            Message::ToggleBlockSelected(index) => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                        if let Some(block) = pane.history.get_mut(index) {
                            block.selected = !block.selected;
                        }
                    }
                }
                Command::none()
            }
            Message::SelectAllBlocks => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                        for block in &mut pane.history {
                            block.selected = true;
                        }
                    }
                }
                Command::none()
            }
            Message::DeselectAllBlocks => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                        for block in &mut pane.history {
                            block.selected = false;
                        }
                    }
                }
                Command::none()
            }
            Message::ExportBlock(index, format) => {
                if let Some(tab) = self.layout.get(self.active_tab) {
                    if let Some(pane) = tab.panes.get(tab.active_pane) {
                        if let Some(block) = pane.history.get(index) {
                            if let Ok(result) = format_blocks(std::slice::from_ref(block), format) {
                                let _ = write_export_file(std::path::Path::new("exports"), format, &result.content);
                                return clipboard::write(result.content);
                            }
                        }
                    }
                }
                Command::none()
            }
            Message::ExportSelected(format) => {
                if let Some(tab) = self.layout.get(self.active_tab) {
                    if let Some(pane) = tab.panes.get(tab.active_pane) {
                        let selected: Vec<Block> = pane.history.iter().filter(|b| b.selected).cloned().collect();
                        if !selected.is_empty() {
                            if let Ok(result) = format_blocks(&selected, format) {
                                let _ = write_export_file(std::path::Path::new("exports"), format, &result.content);
                                return clipboard::write(result.content);
                            }
                        }
                    }
                }
                Command::none()
            }
            Message::CopyBlockExport(index, format) => {
                if let Some(tab) = self.layout.get(self.active_tab) {
                    if let Some(pane) = tab.panes.get(tab.active_pane) {
                        if let Some(block) = pane.history.get(index) {
                            if let Ok(result) = format_blocks(std::slice::from_ref(block), format) {
                                return clipboard::write(result.content);
                            }
                        }
                    }
                }
                Command::none()
            }
            Message::CopySelectedExport(format) => {
                if let Some(tab) = self.layout.get(self.active_tab) {
                    if let Some(pane) = tab.panes.get(tab.active_pane) {
                        let selected: Vec<Block> = pane.history.iter().filter(|b| b.selected).cloned().collect();
                        if !selected.is_empty() {
                            if let Ok(result) = format_blocks(&selected, format) {
                                return clipboard::write(result.content);
                            }
                        }
                    }
                }
                Command::none()
            }
            Message::AiPanelExportConversation(pane_id, format) => {
                if let Some(export) = self.build_ai_conversation_export(
                    AiConversationExportScope::Pane,
                    self.active_tab,
                    Some(pane_id),
                ) {
                    return self.handle_ai_export(export, AiConversationExportScope::Pane, format);
                }
                Command::none()
            }
            Message::AiPanelExportSession(format) => {
                if let Some(export) = self.build_ai_conversation_export(
                    AiConversationExportScope::Session,
                    self.active_tab,
                    None,
                ) {
                    return self.handle_ai_export(export, AiConversationExportScope::Session, format);
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
            Message::FocusSearch => text_input::focus(self.search_input_id.clone()),
            Message::ToggleSearchSuccess => {
                self.search_success_only = !self.search_success_only;
                if self.search_success_only {
                    self.search_failure_only = false;
                }
                Command::none()
            }
            Message::ToggleSearchFailure => {
                self.search_failure_only = !self.search_failure_only;
                if self.search_failure_only {
                    self.search_success_only = false;
                }
                Command::none()
            }
            Message::ToggleSearchPinned => {
                self.search_pinned_only = !self.search_pinned_only;
                Command::none()
            }
            Message::ClearSearch => {
                self.search_query.clear();
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
                    let data = self.build_ai_request("", "explain_error");
                    let request = data;
                    return Command::perform(
                        async move { send_request(request).await },
                        move |result| match result {
                            Ok(resp) => Message::AiResponse(resp.content),
                            Err(err) => Message::AiResponse(format!("AI error ({})", err)),
                        },
                    );
                }
                Command::none()
            }
            Message::AiSuggestFix => {
                if self.ai_settings.enabled {
                    let data = self.build_ai_request("", "suggest_fix");
                    let request = data;
                    return Command::perform(
                        async move { send_request(request).await },
                        move |result| match result {
                            Ok(resp) => Message::AiResponse(resp.content),
                            Err(err) => Message::AiResponse(format!("AI error ({})", err)),
                        },
                    );
                }
                Command::none()
            }
            Message::AiGenerateCommand => {
                if self.ai_settings.enabled {
                    let data = self.build_ai_request("", "generate_command");
                    let request = data;
                    return Command::perform(
                        async move { send_request(request).await },
                        move |result| match result {
                            Ok(resp) => Message::AiResponse(resp.content),
                            Err(err) => Message::AiResponse(format!("AI error ({})", err)),
                        },
                    );
                }
                Command::none()
            }
            Message::AiSummarizeOutput => {
                if self.ai_settings.enabled {
                    let data = self.build_ai_request("", "summarize_output");
                    let request = data;
                    return Command::perform(
                        async move { send_request(request).await },
                        move |result| match result {
                            Ok(resp) => Message::AiResponse(resp.content),
                            Err(err) => Message::AiResponse(format!("AI error ({})", err)),
                        },
                    );
                }
                Command::none()
            }
            Message::ToggleAiPanel => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                        pane.ai_panel_open = !pane.ai_panel_open;
                    }
                }
                Command::none()
            }
            Message::AiPanelInputChanged(pane_id, value) => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(pane_id) {
                        pane.ai_input = value;
                    }
                }
                Command::none()
            }
            Message::AiPanelSetScope(pane_id, scope) => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(pane_id) {
                        pane.ai_context_scope = scope;
                    }
                }
                Command::none()
            }
            Message::AiPanelHoverCitation(pane_id, block_index) => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(pane_id) {
                        pane.highlighted_block = block_index;
                    }
                }
                Command::none()
            }
            Message::AiPanelJumpToBlock(pane_id, block_index) => {
                let mut scroll_cmd = Command::none();
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(pane_id) {
                        pane.highlighted_block = Some(block_index);
                        pane.follow_mode = false;
                        let total_blocks = pane.history.len().max(1);
                        let ratio = (block_index as f32 / (total_blocks.saturating_sub(1) as f32).max(1.0)).clamp(0.0, 1.0);
                        scroll_cmd = scrollable::snap_to(
                            pane.history_scroll_id.clone(),
                            RelativeOffset { x: 0.0, y: ratio },
                        );
                    }
                }
                scroll_cmd
            }
            Message::AiPanelQuickAction(pane_id, action) => {
                let action_label = match action {
                    AiQuickAction::ExplainError => "Explain Error",
                    AiQuickAction::SummarizeOutput => "Summarize Output",
                    AiQuickAction::GenerateCommand => "Generate Command",
                };
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(pane_id) {
                        pane.ai_input = action_label.to_string();
                    }
                }
                self.update(Message::AiPanelSend(pane_id))
            }
            Message::AiTemplateSelected(pane_id, template_id) => {
                self.apply_ai_template(pane_id, template_id, true);
                Command::none()
            }
            Message::AiOnboardingContinue => {
                self.ai_onboarding_open = false;
                self.app_config.ai_onboarding_seen = true;
                let _ = self.save_app_config();
                self.open_ai_panel_for_active_pane();
                Command::none()
            }
            Message::AiOnboardingSkip => {
                self.ai_onboarding_open = false;
                self.app_config.ai_onboarding_seen = true;
                let _ = self.save_app_config();
                Command::none()
            }
            Message::AiPanelSend(pane_id) => {
                if !self.can_send_ai_request() {
                    return Command::none();
                }

                let (user_text, context, sources, _preview) = if let Some(tab) = self.layout.get(self.active_tab) {
                    if let Some(pane) = tab.panes.get(pane_id) {
                        let user_text = pane.ai_input.trim().to_string();
                        if user_text.is_empty() {
                            return Command::none();
                        }
                        let (context, sources, preview) = self.build_ai_context(pane, pane.ai_context_scope);
                        (user_text, context, sources, preview)
                    } else {
                        return Command::none();
                    }
                } else {
                    return Command::none();
                };

                let request_id = if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(pane_id) {
                        pane.ai_input.clear();
                        pane.ai_pending = true;
                        let limits = Self::plan_limits(self.billing_profile.plan);
                        pane.ai_streaming = limits.allow_streaming;  // Only stream if plan allows it
                        pane.ai_request_id = pane.ai_request_id.wrapping_add(1);
                        let request_id = pane.ai_request_id;
                        let user_message = AiChatMessage {
                            role: AiChatRole::User,
                            content: user_text.clone(),
                            sources: sources.clone(),
                        };
                        let assistant = AiChatMessage {
                            role: AiChatRole::Assistant,
                            content: String::new(),
                            sources,
                        };
                        pane.ai_last_redactions.clear();
                        pane.ai_last_redacted_preview = None;
                        pane.ai_chat.push(user_message);
                        pane.ai_chat.push(assistant);
                        pane.ai_stream_target = Some(pane.ai_chat.len().saturating_sub(1));
                        request_id
                    } else {
                        return Command::none();
                    }
                } else {
                    return Command::none();
                };

                let mut request = self.build_ai_prompt("respond", &user_text, &context);
                let request_chars = request.user.chars().count();
                if self.ai_settings.redact_secrets {
                    let redacted = self.redact_prompt(&request.user);
                    if let Some(tab) = self.layout.get_mut(self.active_tab) {
                        if let Some(pane) = tab.panes.get_mut(pane_id) {
                            pane.ai_last_redactions = redacted.matches.clone();
                            pane.ai_last_redacted_preview = Some(redacted.redacted.clone());
                        }
                    }
                    if !self.ai_settings.allow_sensitive && !self.layout.get(self.active_tab).and_then(|tab| tab.panes.get(pane_id)).map(|pane| pane.ai_redaction_override).unwrap_or(false) {
                        request.user = redacted.redacted;
                    }
                }
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(pane_id) {
                        pane.ai_request_started_at = Some(Utc::now());
                        pane.ai_request_chars = request_chars;
                        pane.ai_request_provider = Some(request.provider.clone());
                        pane.ai_request_model = Some(request.model.clone());
                    }
                }
                Command::perform(
                    async move { send_request(request).await },
                    move |result| match result {
                        Ok(resp) => Message::AiResponseReceived(pane_id, request_id, resp.content),
                        Err(err) => Message::AiResponseReceived(pane_id, request_id, format!("AI error ({})", err)),
                    },
                )
            }
            Message::AiPanelToggleRedactionOverride(pane_id, allow) => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(pane_id) {
                        pane.ai_redaction_override = allow;
                    }
                }
                Command::none()
            }
            Message::AiPanelReloadRedactionRules => {
                let _ = self.load_redaction_config();
                Command::none()
            }
            Message::AiPanelOpenRedactionPreview(pane_id) => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(pane_id) {
                        pane.ai_redaction_override = true;
                    }
                }
                Command::none()
            }
            Message::AiPanelCloseRedactionPreview(pane_id) => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(pane_id) {
                        pane.ai_redaction_override = false;
                    }
                }
                Command::none()
            }
            Message::AiPanelStop(pane_id) => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(pane_id) {
                        pane.ai_streaming = false;
                        pane.ai_pending = false;
                        pane.ai_stream_remaining.clear();
                        pane.ai_stream_target = None;
                    }
                }
                Command::none()
            }
            Message::AiResponseReceived(pane_id, request_id, response) => {
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(pane) = tab.panes.get_mut(pane_id) {
                        if pane.ai_request_id == request_id {
                            pane.ai_pending = false;
                            
                            // Check if response is an error
                            let is_error = response.starts_with("AI error") || response.contains("Missing API key");
                            
                            if is_error {
                                // Show error in a friendly way
                                let error_msg = if response.contains("Missing API key") {
                                    "❌ Sorry, I couldn't connect. Please check your API key in .env file.".to_string()
                                } else {
                                    format!("❌ {}", response.replace("AI error", "Error"))
                                };
                                pane.ai_stream_remaining = error_msg;
                            } else {
                                pane.ai_stream_remaining = response.clone();
                            }
                            
                            // For non-streaming (Free plan), show response immediately
                            let limits = Self::plan_limits(self.billing_profile.plan);
                            if !limits.allow_streaming {
                                if let Some(target) = pane.ai_stream_target {
                                    if let Some(message) = pane.ai_chat.get_mut(target) {
                                        message.content = pane.ai_stream_remaining.clone();
                                    }
                                }
                                pane.ai_stream_remaining.clear();
                                pane.ai_streaming = false;
                                pane.ai_stream_target = None;
                            }
                            
                            let duration_ms = pane
                                .ai_request_started_at
                                .map(|started| (Utc::now() - started).num_milliseconds().max(0) as u64)
                                .unwrap_or(0);
                            
                            // Only log successful responses
                            if !is_error {
                                let response_len = if limits.allow_streaming {
                                    pane.ai_stream_remaining.chars().count()
                                } else {
                                    response.chars().count()
                                };
                                let entry = UsageEntry {
                                    timestamp: Utc::now(),
                                    request_chars: pane.ai_request_chars,
                                    response_chars: response_len,
                                    duration_ms,
                                    provider: pane.ai_request_provider.clone().unwrap_or_else(|| "unknown".to_string()),
                                    model: pane.ai_request_model.clone().unwrap_or_else(|| "unknown".to_string()),
                                };
                                self.usage_ledger.entries.push(entry);
                                let _ = self.save_usage_ledger();
                                let limits = Self::plan_limits(self.billing_profile.plan);
                                self.usage_snapshot = self.compute_usage_snapshot(&limits);
                            }
                        }
                    }
                }
                Command::none()
            }
            Message::AiResponse(response) => {
                self.ai_response = Some(response);
                Command::none()
            }
            Message::ToggleAiEnabled => {
                self.ai_settings.enabled = !self.ai_settings.enabled;
                Command::none()
            }
            Message::UpdateAiSettings(_) => Command::none(),
            Message::WindowFocused => {
                debug!("Window focused");
                Command::none()
            }
            Message::WindowUnfocused => {
                debug!("Window unfocused");
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
                            if let Err(err) = pty.writer().write_all(cmd.as_bytes()) {
                                error!("Failed to write to PTY: {}", err);
                            }
                            if let Err(err) = pty.writer().flush() {
                                error!("Failed to flush PTY writer: {}", err);
                            }
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
                        let rect = Rectangle {
                            x: 0.0,
                            y: 0.0,
                            width: self.window_size.width,
                            height: self.window_size.height,
                        };
                        let mut path = Vec::new();
                        if let Some(state) = Self::find_split_hit(&tab.root, rect, self.last_cursor_pos, 6.0, &mut path) {
                            self.resize_state = Some(state);
                            return Command::none();
                        }

                        if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                            pane.mouse_button_down = true;
                            let cell_w = self.renderer.cell_size(&self.theme_config).0;
                            let cell_h = self.renderer.cell_size(&self.theme_config).1;
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
                self.last_cursor_pos = position;
                if let Some(tab) = self.layout.get_mut(self.active_tab) {
                    if let Some(state) = self.resize_state.clone() {
                        Self::update_split_ratio(&mut tab.root, &state, position);
                        return Command::none();
                    }
                    if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
                        pane.last_cursor_pos = position;
                        if pane.mouse_button_down {
                            let cell_w = self.renderer.cell_size(&self.theme_config).0;
                            let cell_h = self.renderer.cell_size(&self.theme_config).1;
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
                    self.resize_state = None;
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
            Message::OpenCommandPalette => {
                self.show_command_palette = true;
                self.palette_query.clear();
                self.palette_selected = 0;
                Command::none()
            }
            Message::CloseCommandPalette => {
                self.show_command_palette = false;
                Command::none()
            }
            Message::UpdatePaletteQuery(query) => {
                self.palette_query = query;
                self.palette_selected = 0; // Reset selection
                Command::none()
            }
            Message::UpdatePaletteSelection(delta) => {
                let actions = self.get_available_actions();
                let filtered_count = actions.iter()
                    .filter(|(name, _)| name.to_lowercase().contains(&self.palette_query.to_lowercase()))
                    .count();
                if filtered_count > 0 {
                    self.palette_selected = ((self.palette_selected as isize + delta) % filtered_count as isize) as usize;
                    if self.palette_selected >= filtered_count {
                        self.palette_selected = filtered_count - 1;
                    }
                }
                Command::none()
            }
            Message::ExecutePaletteAction(action) => {
                self.show_command_palette = false;
                self.execute_palette_action(action);
                Command::none()
            }
            Message::ExportTheme => {
                if let Err(e) = self.export_theme() {
                    error!("Failed to export theme: {}", e);
                }
                Command::none()
            }
            Message::ImportTheme => {
                if let Err(e) = self.import_theme() {
                    error!("Failed to import theme: {}", e);
                }
                Command::none()
            }
            Message::OpenBilling => {
                self.show_billing = true;
                Command::none()
            }
            Message::CloseBilling => {
                self.show_billing = false;
                Command::none()
            }
            Message::BillingUpgradeRequested => {
                self.export_toast = Some(ExportToast {
                    message: "Upgrade flow stubbed: connect Stripe integration later.".to_string(),
                    expires_at: Utc::now() + chrono::Duration::seconds(6),
                });
                Command::none()
            }
            Message::BillingPlanSelected(plan) => {
                self.billing_profile.plan = plan;
                self.app_config.plan_tier = plan;
                let limits = Self::plan_limits(plan);
                self.usage_snapshot = self.compute_usage_snapshot(&limits);
                let _ = self.save_app_config();
                Command::none()
            }
            Message::BillingSyncRequested => {
                self.export_toast = Some(ExportToast {
                    message: "Billing sync stubbed: webhook integration pending.".to_string(),
                    expires_at: Utc::now() + chrono::Duration::seconds(6),
                });
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
            self.renderer.view(&[], &None, "", &self.search_query, self.search_success_only, self.search_failure_only, self.search_pinned_only, self.search_input_id.clone(), dummy_parser.screen(), false, &self.ai_settings, &self.ai_response, 0, None, None, &self.render_cache, &self.row_hashes, 0, 0, &self.theme_config, &self.layout, self.active_tab, self.renaming_tab, &self.rename_buffer, self.history_search_active, &self.history_search_query, &self.history_matches, self.history_selected, false, AiContextScope::LastNBlocks, &[], "", false, false, AiContextPreview { block_count: 0, char_count: 0, token_estimate: 0 }, None, scrollable::Id::unique(), false, &[], None, None, self.export_toast.as_ref(), self.billing_profile.plan, Self::plan_limits(self.billing_profile.plan), self.usage_snapshot.clone())
        };

        if self.ai_onboarding_open {
            self.render_ai_onboarding()
        } else if self.show_billing {
            self.render_billing()
        } else if self.show_command_palette {
            self.render_command_palette()
        } else {
            layout_view
        }
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
                    Message::KeyboardEvent(key, modifiers, text.map(|value| value.to_string()))
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
                    let ai_preview = self.resolve_context_preview(pane, pane.ai_context_scope);
                    let view = self.renderer.view(&pane.history, &pane.current_block, &pane.current_command, &self.search_query, self.search_success_only, self.search_failure_only, self.search_pinned_only, self.search_input_id.clone(), pane.parser.screen(), pane.parser.is_alt_screen_active(), &self.ai_settings, &self.ai_response, pane.scroll_offset, pane.selection_start, pane.selection_end, &self.render_cache, &self.row_hashes, self.active_tab, *pane_id, &self.theme_config, &self.layout, self.active_tab, self.renaming_tab, &self.rename_buffer, self.history_search_active, &self.history_search_query, &self.history_matches, self.history_selected, pane.ai_panel_open, pane.ai_context_scope, &pane.ai_chat, &pane.ai_input, pane.ai_pending, pane.ai_streaming, ai_preview, pane.highlighted_block, pane.history_scroll_id.clone(), pane.ai_redaction_override, &pane.ai_last_redactions, pane.ai_last_redacted_preview.as_deref(), pane.ai_selected_template, self.export_toast.as_ref(), self.billing_profile.plan, Self::plan_limits(self.billing_profile.plan), self.usage_snapshot.clone());
                    let is_active = self
                        .layout
                        .get(self.active_tab)
                        .map(|tab| tab.active_pane == *pane_id)
                        .unwrap_or(false);
                    let border_color = if is_active {
                        Color::from_rgb(0.45, 0.75, 1.0)
                    } else {
                        Color::from_rgb(0.2, 0.2, 0.2)
                    };
                    container(view)
                        .style(move |_theme: &Theme| container::Appearance {
                            background: Some(Background::Color(Color::from_rgb(0.11, 0.11, 0.11))),
                            border: Border {
                                color: border_color,
                                width: if is_active { 2.0 } else { 1.0 },
                                radius: 6.0.into(),
                            },
                            ..Default::default()
                        })
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into()
                } else {
                    let dummy_parser = TerminalParser::new(24, 80);
                    self.renderer.view(&[], &None, "", &self.search_query, self.search_success_only, self.search_failure_only, self.search_pinned_only, self.search_input_id.clone(), dummy_parser.screen(), false, &self.ai_settings, &self.ai_response, 0, None, None, &self.render_cache, &self.row_hashes, self.active_tab, *pane_id, &self.theme_config, &self.layout, self.active_tab, self.renaming_tab, &self.rename_buffer, self.history_search_active, &self.history_search_query, &self.history_matches, self.history_selected, false, AiContextScope::LastNBlocks, &[], "", false, false, AiContextPreview { block_count: 0, char_count: 0, token_estimate: 0 }, None, scrollable::Id::unique(), false, &[], None, None, self.export_toast.as_ref(), self.billing_profile.plan, Self::plan_limits(self.billing_profile.plan), self.usage_snapshot.clone())
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
    let args: Vec<String> = std::env::args().collect();
    let mut log_builder = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    );
    if args.iter().any(|arg| arg == "--verbose") {
        log_builder.filter_level(log::LevelFilter::Debug);
    }
    log_builder.init();

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
