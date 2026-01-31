## Description
Add ability to copy command output in addition to copying commands.

## Current State
- Copy command button exists (line 357 in renderer.rs)
- Copies command text to clipboard
- No way to copy output
- Text selection works but limited

## Requirements
1. **Copy Output Button**
   - Add "Copy Output" button to block UI
   - Next to existing "Copy" button
   - Copies entire output to clipboard

2. **Selective Copy**
   - Copy specific lines of output
   - Copy with/without ANSI codes
   - Copy as plain text or formatted

3. **Keyboard Shortcuts**
   - Cmd+Shift+C to copy output
   - Cmd+C to copy selection
   - Context menu with copy options

4. **Copy Options**
   - Copy command
   - Copy output
   - Copy both (command + output)
   - Copy as markdown code block

## Implementation Details
Add new message and handler:

```rust
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
```

Add button to block UI in renderer.rs.

## Files to Modify
- src/main.rs: Add Message::CopyOutput variant
- src/main.rs: Add handler (similar to CopyCommand)
- src/renderer.rs: Lines 356-361 (add Copy Output button)

## Priority
Medium - Useful for debugging and sharing output
