use crate::Block;
use chrono::Utc;
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
