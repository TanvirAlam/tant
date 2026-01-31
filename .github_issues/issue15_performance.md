## Description
Reduce excessive debug logging that may impact performance, especially eprintln! statements in hot paths.

## Current State
- Many eprintln! statements throughout code
- Debug logging on every event (line 1025: "Event received")
- Logging on every PTY data receive (line 536)
- Logging on every key press (lines 624, 629, 640)
- Canvas rendering logs on every frame (line 523)

## Performance Impact
- stderr I/O on every event is expensive
- Especially bad in tight loops (rendering, PTY reading)
- Can cause stuttering and slowdowns
- Production builds should be quieter

## Requirements
1. **Add Logging Levels**
   - Use proper logging crate (log, tracing, env_logger)
   - Debug, Info, Warn, Error levels
   - Configurable via environment variable

2. **Remove Hot Path Logging**
   - Remove or gate logging in:
     - PTY data reading
     - Event processing
     - Rendering (canvas drawing)
     - Mouse movement

3. **Keep Important Logs**
   - Keep error logging
   - Keep shell integration events (optional)
   - Keep block detection (optional)
   - Add --verbose flag for debugging

4. **Add Proper Error Handling**
   - Replace eprintln! with proper error handling
   - Use Result types
   - Log errors properly

## Implementation Details
```toml
# Cargo.toml
log = "0.4"
env_logger = "0.11"
```

```rust
// Initialize in main
env_logger::Builder::from_env(
    env_logger::Env::default().default_filter_or("info")
).init();

// Replace eprintln! with:
log::debug!("Received {} bytes from PTY", data.len());
log::info!("[Block Detection] Command ended with status {}", status);
log::error!("Failed to write to PTY: {}", err);
```

## Files to Modify
- src/main.rs: Add logging initialization
- src/main.rs: Replace all eprintln! with log macros
- src/renderer.rs: Remove canvas rendering logs (line 523)
- src/parser.rs: Gate debug logs
- Cargo.toml: Add logging dependencies

## Priority
High - Performance impact on user experience

## Testing
Before:
```bash
cargo run 2>&1 | wc -l  # Count log lines
```

After:
```bash
RUST_LOG=error cargo run 2>&1 | wc -l  # Should be much less
RUST_LOG=debug cargo run 2>&1 | wc -l  # Same as before
```
