## Description
Implement the close pane functionality that is currently stubbed in the command palette.

## Current State
- Close pane action exists in command palette but only logs to stderr
- No way to close individual panes
- When running split panes, need ability to close them

## Requirements
1. **Close Current Pane**
   - Keyboard shortcut: Cmd+W or Ctrl+Shift+W
   - Close the currently active pane
   - Redistribute space to remaining panes

2. **Safety Checks**
   - If it's the last pane in a tab, don't close (or close the tab)
   - Prompt if pane has running process
   - Clean up PTY resources properly

3. **Layout Management**
   - Update layout tree when pane is closed
   - Adjust focus to adjacent pane
   - Rebalance pane sizes

## Implementation Details
- Update execute_palette_action in src/main.rs (line 364)
- Implement pane cleanup logic
- Handle PTY shutdown gracefully
- Update active_pane tracking

## Files to Modify
- src/main.rs: Line 364 and pane management logic
- src/pty.rs: May need shutdown method

## Priority
High - Required after split pane is implemented
