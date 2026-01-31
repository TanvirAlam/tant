## Description
Complete the implementation of running pinned commands from the command palette.

## Current State
- Pin/Unpin functionality works (toggle button in block UI)
- Pinned commands appear in command palette
- Executing pinned command is stubbed (line 378 in main.rs)

## Requirements
1. **Execute Pinned Command**
   - Send command to PTY when selected from palette
   - Automatically submit (add carriage return)
   - Show feedback that command was executed

2. **Quick Access**
   - Show pinned commands at top of palette
   - Add keyboard shortcuts for frequent commands
   - Consider dedicated pinned commands panel

3. **Command Management**
   - Edit pinned commands
   - Reorder pinned commands
   - Export/import pinned commands

## Implementation Details
- Fix line 378 in src/main.rs to actually send command to PTY
- Similar to RerunCommand handler (line 694-706)

## Files to Modify
- src/main.rs: Lines 372-384 (RunPinnedCommand handler)

## Code Fix Needed
```rust
PaletteAction::RunPinnedCommand(index) => {
    if let Some(tab) = self.layout.get_mut(self.active_tab) {
        if let Some(pane) = tab.panes.get_mut(tab.active_pane) {
            if let Some(block) = pane.history.get(index) {
                if block.pinned {
                    if let Ok(mut pty) = pane.pty.try_lock() {
                        let cmd = format!("{}\\r", block.command);
                        pty.writer().write_all(cmd.as_bytes()).ok();
                        pty.writer().flush().ok();
                    }
                }
            }
        }
    }
}
```

## Priority
Medium - Improves workflow efficiency
