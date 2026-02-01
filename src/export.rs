use crate::{AiChatRole, AiCitation, Block};
use crate::parser::GitStatus;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    Markdown,
    Json,
    Html,
    Text,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AiConversationExportScope {
    Pane,
    Session,
}

#[derive(Debug, Clone, Serialize)]
pub struct AiConversationMetadata {
    pub exported_at: DateTime<Utc>,
    pub scope: AiConversationExportScope,
    pub tab_title: String,
    pub pane_title: Option<String>,
    pub working_directory: Option<String>,
    pub host: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AiConversationMessage {
    pub role: AiChatRole,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub sources: Vec<AiCitation>,
    pub pane_title: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AiReferencedBlock {
    pub pane_id: usize,
    pub pane_title: String,
    pub block_index: usize,
    pub command: String,
    pub output: String,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
    pub git_status: Option<GitStatus>,
    pub host: String,
    pub is_remote: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AiConversationExport {
    pub metadata: AiConversationMetadata,
    pub messages: Vec<AiConversationMessage>,
    pub referenced_blocks: Vec<AiReferencedBlock>,
}

pub struct ExportResult {
    pub content: String,
}

pub fn format_blocks(blocks: &[Block], format: ExportFormat) -> Result<ExportResult, String> {
    match format {
        ExportFormat::Markdown => Ok(ExportResult {
            content: export_blocks_markdown(blocks),
        }),
        ExportFormat::Json => {
            let content = serde_json::to_string_pretty(blocks).map_err(|e| e.to_string())?;
            Ok(ExportResult { content })
        }
        ExportFormat::Html => Ok(ExportResult {
            content: export_blocks_html(blocks),
        }),
        ExportFormat::Text => Ok(ExportResult {
            content: export_blocks_text(blocks),
        }),
    }
}

pub fn write_export_file(base_dir: &Path, format: ExportFormat, content: &str) -> Result<PathBuf, String> {
    fs::create_dir_all(base_dir).map_err(|e| e.to_string())?;
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let extension = match format {
        ExportFormat::Markdown => "md",
        ExportFormat::Json => "json",
        ExportFormat::Html => "html",
        ExportFormat::Text => "txt",
    };
    let filename = format!("tant_export_{}.{}", timestamp, extension);
    let path = base_dir.join(filename);
    fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(path)
}

pub fn write_ai_export_file(base_dir: &Path, scope: AiConversationExportScope, format: ExportFormat, content: &str) -> Result<PathBuf, String> {
    fs::create_dir_all(base_dir).map_err(|e| e.to_string())?;
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let extension = match format {
        ExportFormat::Markdown => "md",
        ExportFormat::Json => "json",
        ExportFormat::Html => "html",
        ExportFormat::Text => "txt",
    };
    let scope_label = match scope {
        AiConversationExportScope::Pane => "pane",
        AiConversationExportScope::Session => "session",
    };
    let filename = format!("tant_ai_export_{}_{}.{}", scope_label, timestamp, extension);
    let path = base_dir.join(filename);
    fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(path)
}

pub fn format_ai_conversation_export(export: &AiConversationExport, format: ExportFormat) -> Result<ExportResult, String> {
    match format {
        ExportFormat::Markdown => Ok(ExportResult {
            content: export_ai_markdown(export),
        }),
        ExportFormat::Json => {
            let content = serde_json::to_string_pretty(export).map_err(|e| e.to_string())?;
            Ok(ExportResult { content })
        }
        ExportFormat::Text => Ok(ExportResult {
            content: export_ai_text(export),
        }),
        ExportFormat::Html => Err("HTML export is not supported for AI conversations".to_string()),
    }
}

fn export_blocks_markdown(blocks: &[Block]) -> String {
    let mut out = String::new();
    for block in blocks {
        let exit_code = block.exit_code.unwrap_or(-1);
        let duration = block.duration_ms.unwrap_or(0);
        out.push_str(&format!(
            "## {}\n\n```bash\n{}\n```\n\n### Output\n\n```\n{}\n```\n\nExit Code: {}\nDuration: {}ms\n\n",
            block.command, block.command, block.output, exit_code, duration
        ));
    }
    out
}

fn export_blocks_text(blocks: &[Block]) -> String {
    let mut out = String::new();
    for block in blocks {
        out.push_str(&format!("Command: {}\n", block.command));
        out.push_str(&format!("Output:\n{}\n", block.output));
        out.push_str(&format!("Exit Code: {}\n", block.exit_code.unwrap_or(-1)));
        out.push_str(&format!("Duration: {}ms\n", block.duration_ms.unwrap_or(0)));
        out.push_str("\n---\n\n");
    }
    out
}

fn export_blocks_html(blocks: &[Block]) -> String {
    let mut body = String::new();
    for block in blocks {
        let exit_code = block.exit_code.unwrap_or(-1);
        let duration = block.duration_ms.unwrap_or(0);
        body.push_str(&format!(
            "<section class=\"block\">\n<h2>{}</h2>\n<h3>Command</h3>\n<pre><code>{}</code></pre>\n<h3>Output</h3>\n<pre><code>{}</code></pre>\n<p>Exit Code: {} | Duration: {}ms</p>\n</section>\n",
            html_escape(&block.command),
            html_escape(&block.command),
            html_escape(&block.output),
            exit_code,
            duration
        ));
    }
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><style>body{{font-family:monospace;background:#111;color:#eee}}pre{{background:#1b1b1b;padding:12px;border-radius:6px}}</style></head><body>{}</body></html>",
        body
    )
}

fn export_ai_markdown(export: &AiConversationExport) -> String {
    let mut out = String::new();
    out.push_str("# AI Conversation Export\n\n");
    out.push_str(&format!("- Exported: {}\n", export.metadata.exported_at.to_rfc3339()));
    out.push_str(&format!("- Scope: {:?}\n", export.metadata.scope));
    out.push_str(&format!("- Tab: {}\n", export.metadata.tab_title));
    if let Some(pane_title) = &export.metadata.pane_title {
        out.push_str(&format!("- Pane: {}\n", pane_title));
    }
    if let Some(working_directory) = &export.metadata.working_directory {
        out.push_str(&format!("- Working Directory: {}\n", working_directory));
    }
    out.push_str(&format!("- Host: {}\n\n", export.metadata.host));

    out.push_str("## Messages\n\n");
    for message in &export.messages {
        let role_label = match message.role {
            AiChatRole::User => "User",
            AiChatRole::Assistant => "Assistant",
        };
        out.push_str(&format!("### {} ({})\n\n", role_label, message.created_at.to_rfc3339()));
        if let Some(pane_title) = &message.pane_title {
            out.push_str(&format!("*Pane: {}*\n\n", pane_title));
        }
        out.push_str("```text\n");
        out.push_str(&message.content);
        out.push_str("\n```\n\n");
        if message.sources.is_empty() {
            out.push_str("Sources: none\n\n");
        } else {
            out.push_str("Sources:\n");
            for source in &message.sources {
                let block_ref = source
                    .block_index
                    .map(|index| format!("block #{}", index + 1))
                    .unwrap_or_else(|| "current command".to_string());
                out.push_str(&format!("- {} ({})\n", source.label, block_ref));
            }
            out.push('\n');
        }
    }

    if !export.referenced_blocks.is_empty() {
        out.push_str("## Referenced Blocks\n\n");
        for block in &export.referenced_blocks {
            out.push_str(&format!(
                "### Pane: {} • Block #{}\n\n",
                block.pane_title,
                block.block_index + 1
            ));
            out.push_str(&format!("- Command: {}\n", block.command));
            if let Some(cwd) = &block.cwd {
                out.push_str(&format!("- CWD: {}\n", cwd));
            }
            if let Some(started) = block.started_at {
                out.push_str(&format!("- Started: {}\n", started.to_rfc3339()));
            }
            if let Some(ended) = block.ended_at {
                out.push_str(&format!("- Ended: {}\n", ended.to_rfc3339()));
            }
            out.push_str(&format!("- Exit Code: {}\n", block.exit_code.unwrap_or(-1)));
            out.push_str(&format!("- Duration: {}ms\n", block.duration_ms.unwrap_or(0)));
            out.push_str(&format!("- Host: {}\n", block.host));
            if let Some(branch) = &block.git_branch {
                out.push_str(&format!("- Git Branch: {}\n", branch));
            }
            if let Some(status) = &block.git_status {
                out.push_str(&format!("- Git Status: {:?}\n", status));
            }
            out.push_str("\n```bash\n");
            out.push_str(&block.command);
            out.push_str("\n```\n\n### Output\n\n```text\n");
            out.push_str(&block.output);
            out.push_str("\n```\n\n");
        }
    }
    out
}

fn export_ai_text(export: &AiConversationExport) -> String {
    let mut out = String::new();
    out.push_str("AI Conversation Export\n");
    out.push_str(&format!("Exported: {}\n", export.metadata.exported_at.to_rfc3339()));
    out.push_str(&format!("Scope: {:?}\n", export.metadata.scope));
    out.push_str(&format!("Tab: {}\n", export.metadata.tab_title));
    if let Some(pane_title) = &export.metadata.pane_title {
        out.push_str(&format!("Pane: {}\n", pane_title));
    }
    if let Some(working_directory) = &export.metadata.working_directory {
        out.push_str(&format!("Working Directory: {}\n", working_directory));
    }
    out.push_str(&format!("Host: {}\n\n", export.metadata.host));

    out.push_str("Messages:\n\n");
    for message in &export.messages {
        let role_label = match message.role {
            AiChatRole::User => "User",
            AiChatRole::Assistant => "Assistant",
        };
        out.push_str(&format!("{} ({})\n", role_label, message.created_at.to_rfc3339()));
        if let Some(pane_title) = &message.pane_title {
            out.push_str(&format!("Pane: {}\n", pane_title));
        }
        out.push_str(&message.content);
        out.push('\n');
        if message.sources.is_empty() {
            out.push_str("Sources: none\n");
        } else {
            out.push_str("Sources:\n");
            for source in &message.sources {
                let block_ref = source
                    .block_index
                    .map(|index| format!("block #{}", index + 1))
                    .unwrap_or_else(|| "current command".to_string());
                out.push_str(&format!("- {} ({})\n", source.label, block_ref));
            }
        }
        out.push_str("\n---\n\n");
    }

    if !export.referenced_blocks.is_empty() {
        out.push_str("Referenced Blocks:\n\n");
        for block in &export.referenced_blocks {
            out.push_str(&format!("Pane: {} | Block #{}\n", block.pane_title, block.block_index + 1));
            out.push_str(&format!("Command: {}\n", block.command));
            if let Some(cwd) = &block.cwd {
                out.push_str(&format!("CWD: {}\n", cwd));
            }
            if let Some(started) = block.started_at {
                out.push_str(&format!("Started: {}\n", started.to_rfc3339()));
            }
            if let Some(ended) = block.ended_at {
                out.push_str(&format!("Ended: {}\n", ended.to_rfc3339()));
            }
            out.push_str(&format!("Exit Code: {}\n", block.exit_code.unwrap_or(-1)));
            out.push_str(&format!("Duration: {}ms\n", block.duration_ms.unwrap_or(0)));
            out.push_str(&format!("Host: {}\n", block.host));
            if let Some(branch) = &block.git_branch {
                out.push_str(&format!("Git Branch: {}\n", branch));
            }
            if let Some(status) = &block.git_status {
                out.push_str(&format!("Git Status: {:?}\n", status));
            }
            out.push_str("Output:\n");
            out.push_str(&block.output);
            out.push_str("\n\n---\n\n");
        }
    }
    out
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/__tests__/export_tests.rs"));
}
