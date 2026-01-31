## Description
Currently, prompts are showing in the terminal screen which is good, but the block-based UI should be enhanced to match Warp's seamless experience.

## Current State
- Prompts display correctly in terminal (mentioned by user: "so far so good")
- Shell integration with OSC 133 sequences working
- Block detection is accurate
- Live terminal output shows in blocks

## Requirements
1. **Prompt Handling**
   - Ensure prompts appear in live terminal view
   - Don't duplicate prompts in block history
   - Show current prompt above input box

2. **Block Transitions**
   - Smooth transition from live terminal to block
   - Clear visual separation between blocks
   - Maintain prompt visibility during command execution

3. **Visual Polish**
   - Match Warp's clean block separation
   - Proper spacing between blocks
   - Color-coded prompts (success/error)

4. **Input Focus**
   - Prompt should be visible above input area
   - Input area always accessible at bottom
   - Visual connection between prompt and input

## Implementation Details
- Review block rendering logic in src/renderer.rs
- Ensure OSC 133;A (prompt start) handling is correct
- Improve visual styling of prompt area

## Files to Modify
- src/renderer.rs: Lines 180-301 (render_blocks)
- src/main.rs: ParserEvent::PromptShown handling (line 544)

## Priority
Medium - UX enhancement for better user experience
