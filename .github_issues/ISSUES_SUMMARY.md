# Tant Terminal - GitHub Issues Summary

## Overview
Created 15 comprehensive GitHub issues to track improvements needed to make Tant behave similar to Warp terminal.

## Current State Analysis
✅ **Working Well:**
- Prompts showing in terminal screen
- OSC 133 shell integration working correctly
- Block detection is accurate
- Command history with blocks
- Basic UI with input area
- Session save/load
- Command palette (Ctrl+K)

## Issues Created (Priority Order)

### High Priority - Core Functionality

#### #7 - Split Pane Functionality (Horizontal & Vertical)
- **Status:** TODO in code (line 356-365)
- **Impact:** Critical for multi-pane workflow
- Horizontal and vertical splits
- Pane navigation and resizing
- Visual indicators for active pane

#### #8 - Close Pane Functionality
- **Status:** TODO in code (line 364)
- **Impact:** Required after split panes
- Safe pane closure with cleanup
- Layout tree management
- PTY resource cleanup

#### #6 - Performance Optimization - Reduce Debug Logging
- **Status:** Many eprintln! calls in hot paths
- **Impact:** Performance degradation
- Replace with proper logging framework
- Remove hot path logging (PTY reads, rendering)
- Add RUST_LOG environment variable support

### Medium Priority - User Experience

#### #10 - Tab Management System
- **Status:** Basic structure exists, incomplete
- **Impact:** Multi-session workflow
- Create/close/rename tabs
- Tab bar UI
- Keyboard shortcuts (Cmd+1-9, Cmd+T)

#### #9 - Run Pinned Commands
- **Status:** TODO in code (line 378)
- **Impact:** Workflow efficiency
- Currently stubbed, easy fix provided in issue
- Send pinned commands to PTY
- Quick access from command palette

#### #11 - Prompt Display Enhancement
- **Status:** Working but can be improved
- **Impact:** Visual polish
- Better block transitions
- Seamless Warp-like experience
- Visual separation between blocks

#### #3 - Search and Filter Command History
- **Status:** Infrastructure exists, not integrated
- **Impact:** Finding historical commands
- Search by command, output, exit code
- Advanced filters and regex
- Keyboard shortcut (Cmd+F)

#### #4 - Copy Output Functionality
- **Status:** Copy command exists, output missing
- **Impact:** Debugging and sharing
- Copy output button
- Multiple copy options (command, output, both)
- Markdown export format

#### #15 - Working Directory Detection
- **Status:** Infrastructure exists (ParserEvent::Directory)
- **Impact:** Context awareness
- OSC 7 sequence support in shell integration
- Real-time directory tracking
- Show in metadata bar

### Low Priority - Nice to Have

#### #1 - Git Branch Detection
- **Status:** Infrastructure exists (ParserEvent::GitBranch)
- **Impact:** Visual enhancement
- Automatic branch detection
- Visual indicators with 🌿 icon
- Two implementation options (shell integration or git2-rs)

#### #2 - Real Hostname Detection
- **Status:** Hardcoded "localhost" (line 573)
- **Impact:** Minor improvement
- System hostname detection
- Remote session awareness
- Easy fix with hostname crate

#### #12 - Command Autocomplete and Suggestions
- **Status:** No autocomplete currently
- **Impact:** Productivity
- Command history search (Ctrl+R)
- Path completion (Tab)
- AI-powered suggestions (optional)

#### #13 - Real AI Integration
- **Status:** Mock implementation (line 342-350)
- **Impact:** Enhancement feature
- OpenAI/Anthropic integration
- Secure API key storage
- Real command suggestions and error explanations

#### #14 - Theme System Enhancement
- **Status:** Basic theme, colors HashMap empty
- **Impact:** Visual customization
- Preset themes (Dracula, Nord, One Dark, etc.)
- Theme editor with live preview
- Complete color palette

#### #5 - Block Sharing and Export
- **Status:** No export functionality
- **Impact:** Sharing and documentation
- Export formats (Markdown, JSON, HTML)
- Share features (Gist, Pastebin)
- Multi-select blocks

## Quick Wins (Easy Fixes)

1. **Issue #9 - Run Pinned Commands** - Code fix provided in issue
2. **Issue #2 - Hostname Detection** - Simple dependency addition
3. **Issue #4 - Copy Output** - Similar to existing copy command
4. **Issue #6 - Logging** - Add logging crate, replace eprintln!

## Implementation Roadmap

### Phase 1: Core Terminal Features (High Priority)
1. Split Pane (#7)
2. Close Pane (#8)
3. Performance/Logging (#6)

### Phase 2: Workflow Enhancements (Medium Priority)
1. Tab Management (#10)
2. Pinned Commands (#9)
3. Search History (#3)
4. Copy Output (#4)

### Phase 3: Context Awareness (Medium Priority)
1. Working Directory (#15)
2. Prompt Display (#11)

### Phase 4: Polish & Features (Low Priority)
1. Git Branch (#1)
2. Hostname (#2)
3. Autocomplete (#12)
4. Themes (#14)

### Phase 5: Advanced Features (Optional)
1. Real AI Integration (#13)
2. Block Export/Sharing (#5)

## Comparison with Warp

| Feature | Warp | Tant Current | After All Issues |
|---------|------|--------------|-----------------|
| Command Blocks | ✅ | ✅ | ✅ |
| Shell Integration (OSC 133) | ✅ | ✅ | ✅ |
| Split Panes | ✅ | ❌ | ✅ (#7, #8) |
| Tab Management | ✅ | ⚠️ Partial | ✅ (#10) |
| Command Palette | ✅ | ✅ | ✅ |
| AI Features | ✅ | ⚠️ Mock | ✅ (#13) |
| Block Search | ✅ | ❌ | ✅ (#3) |
| Themes | ✅ | ⚠️ Basic | ✅ (#14) |
| Autocomplete | ✅ | ❌ | ✅ (#12) |
| Git Integration | ✅ | ❌ | ✅ (#1) |
| Session Restore | ✅ | ✅ | ✅ |

## Technical Debt Identified

1. **Performance:** Excessive debug logging in hot paths
2. **TODOs:** 5+ TODO comments in codebase
3. **Hardcoded values:** "localhost", empty colors HashMap
4. **Incomplete features:** Split panes, close pane, pinned commands
5. **Missing infrastructure:** Logging framework, autocomplete, themes

## Next Steps

1. Review and prioritize issues in GitHub
2. Start with quick wins (#9, #2, #4, #6)
3. Implement Phase 1 (core terminal features)
4. Test each feature thoroughly
5. Iterate based on user feedback

## Resources

- GitHub Issues: https://github.com/TanvirAlam/tant/issues
- Warp Reference: https://www.warp.dev/
- All issue details: See `.github_issues/` directory

## Notes

- All infrastructure for many features already exists (shell integration, parser events, layout system)
- Project is well-architected, just needs completion
- Current state is functional - these are enhancements, not bug fixes
- Prompts are displaying correctly as mentioned by user ✅
