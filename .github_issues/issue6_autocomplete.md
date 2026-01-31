## Description
Implement intelligent command autocomplete and suggestions like Warp's AI-powered completions.

## Current State
- Basic text input for commands
- No autocomplete
- No command history search
- No intelligent suggestions

## Requirements
1. **Command History Search**
   - Ctrl+R for reverse search
   - Search through command history
   - Filter as you type
   - Show matching commands with context

2. **Path Completion**
   - Tab completion for file paths
   - Show directory contents
   - Navigate with arrow keys

3. **Command Suggestions**
   - Common commands for current context
   - Git commands when in git repo
   - Recently used commands
   - Frequently used commands

4. **AI-Powered Suggestions** (Optional)
   - Natural language to command
   - Error correction suggestions
   - Command explanation
   - Use existing ai_settings infrastructure

## Implementation Details
- Add command history search widget
- Implement path completion using std::fs
- Add suggestion dropdown UI
- Integrate with AI module (already stubbed)

## Files to Modify
- src/main.rs: Add autocomplete logic
- src/renderer.rs: Add suggestion UI
- Leverage existing AiSettings structure

## Priority
Low - Nice to have feature for productivity

## Note
Some AI infrastructure already exists (lines 95-102 in main.rs), can be leveraged for this feature.
