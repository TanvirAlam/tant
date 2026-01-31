## Description
Replace hardcoded "localhost" with actual hostname detection.

## Current State
- Hostname hardcoded to "localhost" (line 573 in main.rs)
- Hostname shown in metadata bar with 💻 icon
- TODO comment in code

## Requirements
1. **Detect Real Hostname**
   - Get system hostname
   - Handle errors gracefully
   - Cache hostname (doesn't change often)

2. **Remote Hostname**
   - Detect SSH sessions
   - Show remote hostname when SSHed
   - Distinguish local vs remote

3. **Visual Indicators**
   - Different icon for remote sessions
   - Show user@host format
   - Color coding for remote

## Implementation Details
Use standard library to get hostname:

```rust
fn get_hostname() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string())
}
```

Or use system call:
```rust
use std::process::Command;

fn get_hostname() -> String {
    Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "localhost".to_string())
}
```

## Dependencies to Add
```toml
hostname = "0.3"  # Simple hostname crate
```

## Files to Modify
- src/main.rs: Line 573 (replace hardcoded "localhost")
- src/main.rs: Add hostname detection in Block creation

## Priority
Low - Minor improvement, current fallback works
