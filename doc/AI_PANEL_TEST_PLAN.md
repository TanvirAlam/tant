# AI Panel Test Plan

## Automated Test Cases (Manual Assertions)

These are test cases intended for automation; they describe expected behavior and can be scripted later. For now, they are written as verifiable steps/expected results.

### AI Panel Toggle
1. Trigger AI panel toggle via command palette action "Toggle AI Panel".
   - **Expected:** AI panel appears on the right without interrupting terminal input focus.
2. Trigger toggle again.
   - **Expected:** AI panel closes and terminal layout returns to full width.

### Context Scope Selection
1. Open AI panel and click each context scope (Current, Last N, Selected, All).
   - **Expected:** The scope label updates to match the selected scope.

### Chat History Persistence (Per Pane)
1. In Pane A, send a message and receive an AI response.
2. Switch to Pane B, open AI panel, send a different message.
3. Switch back to Pane A.
   - **Expected:** Pane A chat history is preserved; Pane B history is not shown in Pane A.

### Quick Actions
1. Click "Explain Error".
2. Click "Summarize Output".
3. Click "Generate Command".
   - **Expected:** Each action sends a request, adds a user message with the action label, and creates a new assistant message.

### Streaming Response and Cancel
1. Send a message and observe assistant response streaming in increments.
   - **Expected:** Assistant message updates incrementally until complete.
2. While streaming, click "Stop".
   - **Expected:** Streaming stops immediately and no further content is appended.

### Sources Display
1. With context scope "Last N" or "Selected", send a message.
   - **Expected:** Assistant message shows sources (block IDs/timestamps or labels) in the message header.

## Manual Testing Steps

### Build and Run
1. Build:
   - `cargo build`
2. Run:
   - `cargo run`

### UI Manual Verification
1. Open the AI panel:
   - Press `Cmd+I` (macOS) or `Ctrl+I` (Linux/Windows).
   - **Expected:** Right-side AI panel appears; terminal input still accepts typing.
2. Toggle from the command palette:
   - Press `Ctrl+K` to open the palette.
   - Select "Toggle AI Panel".
   - **Expected:** Panel visibility toggles.
3. Send a message:
   - Type a question in the AI panel input and press Enter.
   - **Expected:** A user message bubble appears; an assistant bubble starts streaming.
4. Stop streaming:
   - Click "Stop" while the response is streaming.
   - **Expected:** Streaming halts and response does not change further.
5. Context scope checks:
   - Select each context scope (Current, Last N, Selected, All).
   - Send a message.
   - **Expected:** Sources list in assistant bubble reflects the chosen scope.
6. Per-pane persistence:
   - Split pane (`Cmd+D` horizontal or `Cmd+Shift+D` vertical).
   - Open AI panel in each pane and send distinct messages.
   - **Expected:** Chat histories remain isolated per pane.

### Optional: Block Selection Context
1. Execute two commands to create blocks.
2. Use block selection checkboxes to select one block.
3. Choose "Selected" scope and send a message.
   - **Expected:** Sources show only the selected block label.
