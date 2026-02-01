use crate::export::{format_blocks, ExportFormat};
use crate::Block;
use crate::GitStatus;
use chrono::Utc;

fn sample_block() -> Block {
    Block {
        command: "echo hello".to_string(),
        started_at: Some(Utc::now()),
        ended_at: Some(Utc::now()),
        duration_ms: Some(12),
        exit_code: Some(0),
        cwd: None,
        output_range: None,
        pinned: false,
        tags: vec![],
        selected: false,
        output: "hello".to_string(),
        git_branch: Some("main".to_string()),
        git_status: Some(GitStatus::Clean),
        host: "localhost".to_string(),
        is_remote: false,
        collapsed: false,
    }
}

#[test]
fn export_markdown_contains_command_and_output() {
    let block = sample_block();
    let result = format_blocks(&[block], ExportFormat::Markdown).expect("markdown export");
    assert!(result.content.contains("```bash"));
    assert!(result.content.contains("echo hello"));
    assert!(result.content.contains("hello"));
}

#[test]
fn export_json_is_valid() {
    let block = sample_block();
    let result = format_blocks(&[block], ExportFormat::Json).expect("json export");
    let parsed: serde_json::Value = serde_json::from_str(&result.content).expect("valid json");
    assert!(parsed.is_array());
}

#[test]
fn export_html_escapes_content() {
    let mut block = sample_block();
    block.output = "<tag>&\"'".to_string();
    let result = format_blocks(&[block], ExportFormat::Html).expect("html export");
    assert!(result.content.contains("&lt;tag&gt;"));
    assert!(result.content.contains("&amp;"));
    assert!(result.content.contains("&quot;"));
    assert!(result.content.contains("&#39;"));
}

#[test]
fn export_markdown_includes_exit_code_and_duration_defaults() {
    let mut block = sample_block();
    block.exit_code = None;
    block.duration_ms = None;
    let result = format_blocks(&[block], ExportFormat::Markdown).expect("markdown export");
    assert!(result.content.contains("Exit Code: -1"));
    assert!(result.content.contains("Duration: 0ms"));
}

#[test]
fn export_text_contains_separator_and_fields() {
    let block = sample_block();
    let result = format_blocks(&[block], ExportFormat::Text).expect("text export");
    assert!(result.content.contains("Command: echo hello"));
    assert!(result.content.contains("Output:\nhello"));
    assert!(result.content.contains("Exit Code: 0"));
    assert!(result.content.contains("Duration: 12ms"));
    assert!(result.content.contains("\n---\n"));
}

#[test]
fn export_html_wraps_document_structure() {
    let block = sample_block();
    let result = format_blocks(&[block], ExportFormat::Html).expect("html export");
    assert!(result.content.starts_with("<!doctype html>"));
    assert!(result.content.contains("<section class=\"block\">"));
    assert!(result.content.contains("</html>"));
}

#[test]
fn export_json_preserves_fields() {
    let block = sample_block();
    let result = format_blocks(&[block], ExportFormat::Json).expect("json export");
    let parsed: serde_json::Value = serde_json::from_str(&result.content).expect("valid json");
    let first = &parsed[0];
    assert_eq!(first["command"], "echo hello");
    assert_eq!(first["output"], "hello");
    assert_eq!(first["exit_code"], 0);
    assert_eq!(first["pinned"], false);
}

#[test]
fn export_markdown_handles_multiple_blocks() {
    let mut second = sample_block();
    second.command = "ls -la".to_string();
    second.output = "total 0".to_string();
    let result = format_blocks(&[sample_block(), second], ExportFormat::Markdown).expect("markdown export");
    assert!(result.content.contains("## echo hello"));
    assert!(result.content.contains("## ls -la"));
}

#[test]
fn export_text_defaults_exit_code_and_duration() {
    let mut block = sample_block();
    block.exit_code = None;
    block.duration_ms = None;
    let result = format_blocks(&[block], ExportFormat::Text).expect("text export");
    assert!(result.content.contains("Exit Code: -1"));
    assert!(result.content.contains("Duration: 0ms"));
}

#[test]
fn export_html_escapes_command_and_output() {
    let mut block = sample_block();
    block.command = "echo <hi>".to_string();
    block.output = "& <hi>".to_string();
    let result = format_blocks(&[block], ExportFormat::Html).expect("html export");
    assert!(result.content.contains("echo &lt;hi&gt;"));
    assert!(result.content.contains("&amp; &lt;hi&gt;"));
}

#[test]
fn export_json_includes_optional_fields() {
    let mut block = sample_block();
    block.cwd = Some(std::path::PathBuf::from("/tmp"));
    block.output_range = Some((1, 4));
    block.tags = vec!["tag1".to_string(), "tag2".to_string()];
    block.selected = true;
    block.is_remote = true;
    block.collapsed = true;
    let result = format_blocks(&[block], ExportFormat::Json).expect("json export");
    let parsed: serde_json::Value = serde_json::from_str(&result.content).expect("valid json");
    let first = &parsed[0];
    assert_eq!(first["cwd"], "/tmp");
    assert_eq!(first["output_range"], serde_json::json!([1, 4]));
    assert_eq!(first["tags"], serde_json::json!(["tag1", "tag2"]));
    assert_eq!(first["selected"], true);
    assert_eq!(first["is_remote"], true);
    assert_eq!(first["collapsed"], true);
}

#[test]
fn export_markdown_empty_blocks_returns_empty_string() {
    let result = format_blocks(&[], ExportFormat::Markdown).expect("markdown export");
    assert!(result.content.is_empty());
}

#[test]
fn export_text_multiple_blocks_includes_separators_for_each() {
    let mut second = sample_block();
    second.command = "pwd".to_string();
    second.output = "/tmp".to_string();
    let result = format_blocks(&[sample_block(), second], ExportFormat::Text).expect("text export");
    let separator_count = result.content.matches("\n---\n").count();
    assert_eq!(separator_count, 2);
    assert!(result.content.contains("Command: echo hello"));
    assert!(result.content.contains("Command: pwd"));
}

#[test]
fn export_html_renders_all_blocks() {
    let mut second = sample_block();
    second.command = "uname -a".to_string();
    let result = format_blocks(&[sample_block(), second], ExportFormat::Html).expect("html export");
    let section_count = result.content.matches("<section class=\"block\">").count();
    assert_eq!(section_count, 2);
    assert!(result.content.contains("<h2>echo hello</h2>"));
    assert!(result.content.contains("<h2>uname -a</h2>"));
}

#[test]
fn export_json_empty_blocks_is_empty_array() {
    let result = format_blocks(&[], ExportFormat::Json).expect("json export");
    let parsed: serde_json::Value = serde_json::from_str(&result.content).expect("valid json");
    assert_eq!(parsed, serde_json::json!([]));
}
