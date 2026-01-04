# Tant Shell Integration

This directory contains shell integration scripts that enable Warp-style command block detection in Tant terminal. Instead of relying on heuristics, these scripts use **OSC 133 escape sequences** to reliably mark command boundaries.

## What is Shell Integration?

Shell integration provides accurate command block detection by having your shell emit special escape sequences at key moments:

- **Prompt shown** - When the shell is ready to accept a command
- **Command start** - Right before a command executes
- **Command finished** - After a command completes (includes exit code and duration)

This allows Tant to:
- ✅ Accurately segment terminal output into blocks
- ✅ Track exit codes for each command
- ✅ Measure command execution duration
- ✅ Reliably separate prompts from command output
- ✅ Never miss or incorrectly parse block boundaries

## Installation

### Quick Install (Recommended)

Run the installer script, which will auto-detect your shell:

```bash
cd shell_integration
./install.sh
```

Then restart your shell or source your config file.

### Manual Installation

#### Zsh

Add to your `~/.zshrc`:

```bash
source /path/to/tant/shell_integration/tant.zsh
```

#### Bash

Add to your `~/.bashrc`:

```bash
source /path/to/tant/shell_integration/tant.bash
```

#### Fish

Copy the integration script to Fish's config directory:

```bash
cp tant.fish ~/.config/fish/conf.d/
```

### Install for Specific Shell

```bash
./install.sh --shell zsh   # or bash, fish
```

### Uninstall

```bash
./install.sh --uninstall
```

## How It Works

### OSC 133 Sequences

The shell integration uses **OSC (Operating System Command) 133** sequences, which are standardized escape codes:

- `OSC 133;A` - Prompt start
- `OSC 133;B` - Prompt end / command line start  
- `OSC 133;C` - Command execution start
- `OSC 133;D;EXIT_CODE` - Command finished (with exit code)

These sequences pass through the PTY and are detected by Tant's parser.

### Example Flow

1. **Shell displays prompt**
   ```
   \033]133;A\007  ← Prompt start marker
   user@host:~$
   \033]133;B\007  ← Ready for command input
   ```

2. **User types command and hits Enter**
   ```
   \033]133;C\007  ← Command execution start
   ls -la
   [command output]
   ```

3. **Command finishes**
   ```
   \033]133;D;0\007  ← Command ended with exit code 0
   ```

4. **Back to step 1**

### Integration Points

Each shell script hooks into the shell's execution cycle:

**Zsh:**
- `precmd` hook - runs before each prompt
- `preexec` hook - runs before command execution
- Custom prompt modification

**Bash:**
- `PROMPT_COMMAND` - runs before each prompt
- `DEBUG` trap - runs before command execution

**Fish:**
- `fish_prompt` - wraps the prompt function
- `fish_preexec` event - before command execution
- `fish_postexec` event - after command execution

## Technical Details

### OSC Sequence Format

OSC sequences follow this format:
```
ESC ] <command> ; <params> BEL
```

Where:
- `ESC` = `\033` or `\x1b`
- `]` = Start of OSC
- `BEL` = `\007` (or can use `ESC \` as terminator)

Example: `\033]133;A\007`

### Parser Implementation

In `src/parser.rs`, the terminal parser:

1. Buffers incoming PTY data
2. Searches for OSC 133 sequences
3. Emits `ParserEvent` when markers are detected:
   - `ParserEvent::PromptShown`
   - `ParserEvent::CommandStart`
   - `ParserEvent::CommandEnd(exit_code)`

4. Main application uses these events to create/complete command blocks

### Why Not Heuristics?

Without shell integration, terminal emulators must guess where commands start/end by:
- Detecting prompt patterns (unreliable)
- Watching for command-like strings (false positives)
- Timing-based detection (slow commands confuse this)

Shell integration eliminates this guesswork entirely.

## Compatibility

### Supported Shells

- ✅ Zsh (all recent versions)
- ✅ Bash (4.0+)
- ✅ Fish (3.0+)

### Known Limitations

- Shell integration requires modifying your shell config
- Some prompt themes may need adjustment
- SSH sessions need the integration installed on remote hosts
- Won't work with shells that don't support hooks (e.g., basic `sh`)

### Other Terminal Emulators

The OSC 133 sequences used here are compatible with:
- Warp
- iTerm2
- VS Code integrated terminal
- FinalTerm (original implementation)

## Troubleshooting

### Integration not working?

1. Check if the script is being sourced:
   ```bash
   echo $TANT_SHELL_INTEGRATION
   # Should output: 1
   ```

2. Enable debug mode in your shell to see if markers are emitted

3. Look for error messages when starting a new shell

### Conflicts with other integrations?

If you're using other terminal enhancements (e.g., Warp's shell integration), you may need to disable one or ensure they don't conflict.

### Prompts look weird?

Some prompt themes might not handle the escape sequences correctly. Try:
- Updating your prompt theme
- Using `TANT_USE_SIMPLE_PROMPT=1` environment variable

## Development

### Testing the Integration

1. Source the integration script in your shell
2. Run commands and check for debug output in Tant
3. Look for `[Shell Integration]` messages in stderr

### Adding New Shell Support

To add support for another shell:

1. Create a new script (e.g., `tant.tcsh`)
2. Find the shell's pre/post command hooks
3. Emit OSC 133 sequences at the right times
4. Update `install.sh` to support the new shell

## References

- [FinalTerm Shell Integration](http://www.finalterm.org/)
- [iTerm2 Shell Integration](https://iterm2.com/documentation-shell-integration.html)
- [Warp's approach to command blocks](https://www.warp.dev/)
- [OSC Sequences Specification](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html)

## License

Part of the Tant Terminal project.
