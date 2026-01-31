## Description
Implement automatic git branch detection and display.

## Current State
- Git branch field exists in Block structure (line 41 in main.rs)
- Git branch shown in metadata area (lines 269-275 in renderer.rs)
- ParserEvent::GitBranch exists but not populated
- Currently always None

## Requirements
1. **Automatic Branch Detection**
   - Detect when in git repository
   - Show current branch name
   - Update on branch changes (checkout, merge, etc.)

2. **Visual Indicators**
   - Branch icon (🌿) already present
   - Show branch in metadata bar
   - Color-code by branch (main/master vs feature)
   - Show git status (clean, dirty, conflicts)

3. **Performance**
   - Cache git info to avoid slowdown
   - Async git operations
   - Update on directory change only

## Implementation Options

### Option 1: Shell Integration (Recommended)
- Emit git info via OSC sequence
- Add to shell integration scripts
- Most accurate, no extra overhead

### Option 2: Direct Git Query
- Use git2-rs crate
- Query on directory change
- More flexible, works without shell integration

### Option 3: Parse Prompt
- Extract from prompt if prompt shows branch
- Least reliable but no dependencies

## Implementation Details
**Option 1 (Shell Integration):**
- Add git branch emission to shell integration
- Parse custom OSC sequence in parser.rs
- Handle ParserEvent::GitBranch in main.rs

**Option 2 (Git Library):**
```toml
# Cargo.toml
git2 = "0.18"
```

```rust
// Query git branch
fn get_git_branch(cwd: &Path) -> Option<String> {
    let repo = Repository::open(cwd).ok()?;
    let head = repo.head().ok()?;
    head.shorthand().map(String::from)
}
```

## Files to Modify
- shell_integration/*.{zsh,bash,fish}: Emit git branch
- src/parser.rs: Parse git branch sequence
- src/main.rs: Lines 589-593 (handle GitBranch event)
- Or add src/git.rs for Option 2

## Priority
Low - Nice to have visual enhancement

## Note
Infrastructure already exists (ParserEvent::GitBranch), choose implementation approach.
