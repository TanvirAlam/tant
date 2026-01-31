## Description
Improve working directory detection to dynamically track directory changes during shell session.

## Current State
- Initial working directory set on pane creation
- Working directory shown in metadata area (line 262-266 in renderer.rs)
- ParserEvent::Directory exists but not used by shell integration
- Directory stored in blocks but not updated dynamically

## Requirements
1. **Track Directory Changes**
   - Detect when user runs `cd` command
   - Update working directory in real-time
   - Use OSC sequences for accurate tracking

2. **Shell Integration Enhancement**
   - Add OSC 7 (current directory) to shell integration scripts
   - Format: `\033]7;file://hostname/path\007`
   - Parse in parser.rs

3. **Visual Feedback**
   - Show current directory in metadata bar
   - Update immediately on directory change
   - Show directory in tab title (optional)

4. **Per-Block Directory**
   - Each block stores its working directory
   - Show directory context in block header
   - Useful for understanding command context

## Implementation Details
- Update shell integration scripts (tant.zsh, tant.bash, tant.fish)
- Add OSC 7 emission after `cd` commands
- Parse OSC 7 in parser.rs (similar to OSC 133)
- Handle ParserEvent::Directory in main.rs

## Files to Modify
- shell_integration/tant.zsh: Add OSC 7 emission
- shell_integration/tant.bash: Add OSC 7 emission
- shell_integration/tant.fish: Add OSC 7 emission
- src/parser.rs: Detect OSC 7 sequences
- src/main.rs: Lines 583-588 (handle Directory event)

## Example Shell Integration (zsh)
```bash
# In precmd hook, emit current directory
_tant_precmd() {
    printf "\033]7;file://%s%s\007" "$HOST" "$PWD"
    printf "\033]133;A\007"
}
```

## Priority
Medium - Improves context awareness

## Note
Infrastructure already exists (ParserEvent::Directory), just needs shell integration support.
