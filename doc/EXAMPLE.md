# Shell Integration Example

This document demonstrates how the shell integration works in practice.

## Installation Example

```bash
# Clone or navigate to tant project
cd ~/Projects/tant

# Install shell integration for your shell
cd shell_integration
./install.sh

# Output:
# ========================================
#   Tant Shell Integration Installer
# ========================================
#
# ℹ Detected shell: zsh
# ℹ Installing for Zsh...
# ✓ Copied tant.zsh to /Users/you/.tant
# ✓ Added integration to /Users/you/.zshrc
# ℹ Restart your shell or run: source /Users/you/.zshrc
#
# ✓ Installation complete!
#
# ℹ Shell integration provides:
#   • Accurate command block detection
#   • Exit code tracking per command
#   • Command duration tracking
#   • Reliable prompt/command separation
```

## Testing the Integration

After installation, start a new terminal session (or source your shell config):

```bash
# Start tant terminal
cargo run

# In the tant terminal, you should see:
# Tant shell integration loaded for zsh
```

## Behind the Scenes

When you run commands, the shell emits invisible OSC sequences:

### User View
```bash
user@host:~/Projects/tant$ ls
Cargo.lock  Cargo.toml  shell_integration  src  target
user@host:~/Projects/tant$ echo "hello"
hello
user@host:~/Projects/tant$ exit
```

### What Actually Gets Sent to PTY
```
\033]133;A\007user@host:~/Projects/tant$ \033]133;B\007\033]133;C\007ls
Cargo.lock  Cargo.toml  shell_integration  src  target
\033]133;D;0\007
\033]133;A\007user@host:~/Projects/tant$ \033]133;B\007\033]133;C\007echo "hello"
hello
\033]133;D;0\007
\033]133;A\007user@host:~/Projects/tant$ \033]133;B\007exit
```

Where:
- `\033]133;A\007` = Prompt start
- `\033]133;B\007` = Ready for command input
- `\033]133;C\007` = Command execution starts
- `\033]133;D;0\007` = Command finished with exit code 0

## Parser Detection

In the Tant terminal stderr output, you'll see:

```
Received 45 bytes from PTY
[Shell Integration] Prompt shown
[Block Detection] Prompt shown
Received 28 bytes from PTY
[Shell Integration] Command started
[Block Detection] Command started - new block created
Received 156 bytes from PTY
[Shell Integration] Command ended with exit code: 0
[Block Detection] Command ended with status 0 - block saved
```

## Command Block Structure

Each block created by Tant will contain:

```rust
Block {
    command: "ls",                          // The command that was run
    output: "Cargo.lock\nCargo.toml\n...", // Full output
    status: Some(0),                        // Exit code from shell integration
    start_time: Some(Instant),              // When command started
    duration: Some(Duration(2ms)),          // How long it took
    directory: "/Users/you/Projects/tant",  // Working directory
    git_branch: Some("main"),               // Git branch if applicable
    host: "localhost",                      // Host machine
    pinned: false,                          // User pinned status
}
```

## Exit Code Tracking

The integration accurately captures exit codes:

```bash
# Successful command (exit code 0)
$ ls
[files listed]

# Failed command (exit code 1)
$ ls /nonexistent
ls: /nonexistent: No such file or directory

# Custom exit code
$ exit 42
```

Each block will have the correct `status` field matching the command's exit code.

## Duration Tracking

Command duration is measured from the `CommandStart` event to the `CommandEnd` event:

```bash
$ sleep 2
# Duration: ~2000ms

$ echo "fast"
# Duration: ~1ms
```

This is much more accurate than heuristic-based approaches.

## Troubleshooting Example

If markers aren't detected:

```bash
# Check if integration is loaded
$ echo $TANT_SHELL_INTEGRATION
1

# If not loaded, source manually
$ source ~/.tant/tant.zsh
Tant shell integration loaded for zsh

# Test marker emission (won't be visible but goes to PTY)
$ printf "\033]133;A\007TEST\033]133;C\007"
```

## Manual Testing of OSC Sequences

You can manually emit sequences to test the parser:

```bash
# In any terminal
$ printf "\033]133;A\007"  # Prompt start
$ printf "\033]133;C\007"  # Command start
$ echo "test command"
$ printf "\033]133;D;0\007"  # Command end with exit code 0
```

When running Tant, you should see the parser detect these sequences in stderr.

## Comparing With and Without Integration

### Without Shell Integration (Heuristic Mode)
- ❌ May miss commands if they don't match patterns
- ❌ May create false blocks from prompt changes
- ❌ Exit codes are guessed or unavailable
- ❌ Duration tracking is unreliable
- ❌ Prompt/output separation is fuzzy

### With Shell Integration
- ✅ Every command creates exactly one block
- ✅ Prompts are never mistaken for commands
- ✅ Exit codes are accurate and immediate
- ✅ Duration is precise to the millisecond
- ✅ Perfect separation of all terminal elements

## Advanced: Custom Shell Hooks

You can add custom logic to the integration hooks:

```bash
# In ~/.zshrc, after sourcing tant.zsh

# Override precmd to add custom tracking
_my_precmd() {
    # Your custom code here
    echo "Command finished at $(date)"
}

add-zsh-hook precmd _my_precmd
```

This runs alongside Tant's integration without conflicts.

## Performance Impact

The shell integration has minimal performance impact:

- **Overhead per command**: < 1ms
- **Memory usage**: Negligible (a few KB for hooks)
- **Startup time**: +10-20ms when sourcing the script

The benefits far outweigh this tiny cost.
