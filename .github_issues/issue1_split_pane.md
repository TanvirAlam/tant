## Description
Currently, the split pane actions in the command palette (Ctrl+K) are not implemented. This feature is essential for Warp-like multi-pane terminal experience.

## Current State
- Split pane actions exist in command palette but are stubbed with `// TODO: Implement split pane`
- Layout system with LayoutNode::Split exists but is not used
- Only single pane per tab is supported

## Requirements
1. **Horizontal Split** - Split pane side-by-side
   - Keyboard shortcut: Cmd+D or Ctrl+Shift+D
   - Add new pane to the right of current pane
   - Distribute space equally (50/50)

2. **Vertical Split** - Split pane top-bottom
   - Keyboard shortcut: Cmd+Shift+D or Ctrl+D
   - Add new pane below current pane
   - Distribute space equally (50/50)

3. **Pane Navigation**
   - Cmd+[ / Cmd+] to switch between panes
   - Or Ctrl+Alt+Arrow keys
   - Visual indicator for active pane

4. **Pane Resizing**
   - Drag divider between panes to resize
   - Or keyboard shortcuts (Cmd+Ctrl+Arrow)

## Implementation Details
- Update execute_palette_action in src/main.rs to handle split actions
- Create new PTY for each new pane
- Update layout tree structure properly
- Ensure each pane has its own:
  - PTY instance
  - Command history
  - Working directory
  - Parser state

## Files to Modify
- src/main.rs: Lines 356-365 (execute_palette_action)
- src/main.rs: Add pane navigation logic
- src/renderer.rs: Ensure layout rendering works with multiple panes

## References
- Warp split pane behavior: https://www.warp.dev/
- iTerm2 split pane functionality

## Priority
High - This is a core terminal multiplexer feature
