## Description
Implement search and filter functionality for command history blocks.

## Current State
- Search query field exists (line 211 in main.rs)
- UpdateSearch message handler (lines 747-750)
- Not integrated into UI
- Blocks shown but not filterable

## Requirements
1. **Search UI**
   - Search bar above block history
   - Filter blocks as you type
   - Highlight matching text
   - Show match count

2. **Search Criteria**
   - Search command text
   - Search output content
   - Filter by exit code (success/failure)
   - Filter by date/time range
   - Filter by directory

3. **Advanced Filters**
   - Combine multiple filters
   - Regex support
   - Case sensitive/insensitive
   - Pinned commands only

4. **Keyboard Shortcuts**
   - Cmd+F or Ctrl+F to open search
   - Escape to clear search
   - Enter to jump to next match

## Implementation Details
- Add search UI component in renderer.rs
- Filter history blocks based on query
- Add filter options dropdown
- Highlight matching text in blocks

## Files to Modify
- src/renderer.rs: Add search UI component
- src/renderer.rs: Filter blocks in render_blocks
- src/main.rs: Enhance UpdateSearch handler

## Example Filter Logic
```rust
fn filter_blocks(history: &[Block], query: &str, filters: &SearchFilters) -> Vec<&Block> {
    history.iter()
        .filter(|b| {
            if !query.is_empty() {
                let matches_cmd = b.command.contains(query);
                let matches_output = b.output.contains(query);
                if !(matches_cmd || matches_output) {
                    return false;
                }
            }
            if let Some(exit_code) = filters.exit_code {
                if b.exit_code != Some(exit_code) {
                    return false;
                }
            }
            true
        })
        .collect()
}
```

## Priority
Medium - Important for finding historical commands
