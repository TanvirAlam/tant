## Description
Implement full tab management system similar to Warp's tabs feature.

## Current State
- Basic tab structure exists
- Only one tab is created on startup
- No way to create/close/rename tabs
- Tab switching via command palette works

## Requirements
1. **Create New Tab**
   - Keyboard shortcut: Cmd+T or Ctrl+T
   - Creates new tab with single pane
   - Starts in user's home directory

2. **Close Tab**
   - Keyboard shortcut: Cmd+W (when no panes to close) or Ctrl+Shift+Q
   - Close current tab
   - Prompt if tab has running processes
   - Clean up all PTYs in tab

3. **Switch Tabs**
   - Cmd+1 through Cmd+9 for tabs 1-9
   - Cmd+[ and Cmd+] for prev/next tab
   - Show tab bar at top

4. **Rename Tabs**
   - Double-click tab to rename
   - Or Cmd+Shift+R
   - Show meaningful tab titles

5. **Tab Bar UI**
   - Visual tab bar similar to Warp
   - Show tab titles
   - Indicate active tab
   - Close buttons on tabs

## Implementation Details
- Add tab bar widget to renderer
- Add tab creation/deletion logic
- Handle keyboard shortcuts
- Persist tab state in session

## Files to Modify
- src/main.rs: Add tab management messages and handlers
- src/renderer.rs: Add tab bar rendering

## Priority
Medium - Enhances multi-session workflow
