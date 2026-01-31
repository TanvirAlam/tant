## Description
Add ability to export and share command blocks similar to Warp's sharing features.

## Current State
- Blocks contain all necessary data
- Session save/load exists
- No export of individual blocks
- No sharing functionality

## Requirements
1. **Export Formats**
   - Export as Markdown
   - Export as JSON
   - Export as HTML
   - Export as plain text

2. **Export Scope**
   - Single block
   - Multiple selected blocks
   - Entire session
   - Date range

3. **Share Features**
   - Generate shareable link (requires backend)
   - Copy as formatted snippet
   - Export to Gist
   - Export to Pastebin

4. **Block Selection**
   - Multi-select blocks with checkboxes
   - Cmd+Click to select multiple
   - Select all / deselect all

## Implementation Details
Add export functions:

```rust
fn export_block_markdown(block: &Block) -> String {
    format!(
        "## {}\\n\\n```bash\\n{}\\n```\\n\\n### Output\\n\\n```\\n{}\\n```\\n\\nExit Code: {}\\nDuration: {}ms\\n",
        block.command,
        block.command,
        block.output,
        block.exit_code.unwrap_or(-1),
        block.duration_ms.unwrap_or(0)
    )
}
```

Add to command palette or block UI.

## Files to Modify
- src/main.rs: Add export functionality
- src/renderer.rs: Add export buttons
- Add: src/export.rs for export formats

## Priority
Low - Nice to have for sharing and documentation
