# Keyboard Input Test Guide

## Testing the PTY Keyboard Integration

Run the terminal emulator:
```bash
cargo run
```

### Tests to Perform

#### 1. Normal Text Input
- Type regular characters: `hello world`
- Expected: Characters appear in the terminal

#### 2. Special Keys
- **Enter**: Should execute commands
- **Backspace**: Should delete characters
- **Arrow keys**: Should navigate command history (↑/↓) and cursor (←/→)
- **Tab**: Should trigger shell completion
- **Escape**: Should cancel operations

#### 3. Control Combinations
- **Ctrl+C**: Should send interrupt signal (^C)
- **Ctrl+D**: Should send EOF
- **Ctrl+L**: Should clear screen
- **Ctrl+A**: Move to beginning of line
- **Ctrl+E**: Move to end of line
- **Ctrl+U**: Clear line before cursor
- **Ctrl+K**: Clear line after cursor

#### 4. Bracketed Paste
- Copy multi-line text
- Paste into terminal
- Expected: Pasted content wrapped in escape sequences, preventing auto-execution

## Implementation Details

### Key Mappings (from `main.rs:126-179`)

**Named Keys:**
- Enter → `\r` (0x0d)
- Backspace → `0x7f` (127)
- Tab → `\t` (0x09)
- Escape → `0x1b` (27)
- Arrow Up → `\x1b[A`
- Arrow Down → `\x1b[B`
- Arrow Right → `\x1b[C`
- Arrow Left → `\x1b[D`
- Home → `\x1b[H`
- End → `\x1b[F`
- PageUp → `\x1b[5~`
- PageDown → `\x1b[6~`
- Delete → `\x1b[3~`
- Insert → `\x1b[2~`

**Ctrl Combinations:**
- Ctrl+A-Z → 0x01-0x1a (computed as: `(UPPERCASE - 'A') + 1`)
- Ctrl+Space/@ → 0x00 (NUL)
- Ctrl+[ → 0x1b (ESC)
- Ctrl+\ → 0x1c
- Ctrl+] → 0x1d
- Ctrl+^ → 0x1e
- Ctrl+_ → 0x1f

### Event Flow

```
User types → Iced keyboard event → 
  ↓
subscription (line 524-545) →
  ↓
If text & no modifiers → Message::TextInput
If special key or modifiers → Message::KeyPress
  ↓
update() handler →
  ↓
Write to PTY stdin →
  ↓
Shell processes input →
  ↓
Output read via PTY stdout (Message::Tick polling)
```

## Expected Results

After running the terminal, you should be able to:
1. ✅ Type commands and see them echoed
2. ✅ Press Enter to execute commands
3. ✅ Use Backspace to delete characters
4. ✅ Use arrow keys for navigation and history
5. ✅ Use Ctrl+C to interrupt running commands
6. ✅ Paste multi-line content safely with bracketed paste mode
