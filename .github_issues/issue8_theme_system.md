## Description
Enhance the theme system with more complete color configuration and preset themes.

## Current State
- ThemeConfig structure exists (lines 18-26 in main.rs)
- Import/Export theme functionality exists
- Colors HashMap is empty (line 516: "Will add defaults later")
- Basic dark theme in use

## Requirements
1. **Complete Theme Configuration**
   - Define all necessary colors (foreground, background, cursor, selection, etc.)
   - ANSI color palette (16 colors)
   - UI colors (input, borders, buttons, etc.)
   - Syntax highlighting colors

2. **Preset Themes**
   - Dark themes: Dracula, One Dark, Nord, Tokyo Night
   - Light themes: Light, Solarized Light
   - Import from popular formats (iTerm2, VS Code)

3. **Theme Editor**
   - Visual theme editor in UI
   - Live preview
   - Color picker
   - Export custom themes

4. **Font Configuration**
   - Font family selection
   - Font size
   - Line height
   - Letter spacing
   - Ligature support

## Implementation Details
- Populate colors HashMap with defaults
- Create theme presets
- Add theme selector to command palette
- Implement theme preview

## Files to Modify
- src/main.rs: Lines 510-517 (theme_config initialization)
- src/renderer.rs: Use ThemeConfig colors everywhere
- Add: src/themes.rs for preset themes

## Example Theme Structure
```rust
colors: {
    "foreground": [0.9, 0.9, 0.9],
    "background": [0.12, 0.12, 0.12],
    "cursor": [0.4, 0.7, 0.9],
    "selection": [0.3, 0.3, 0.4],
    "ansi_black": [0.0, 0.0, 0.0],
    // ... 16 ANSI colors
    "ui_border": [0.2, 0.2, 0.2],
    "ui_input_bg": [0.35, 0.35, 0.35],
    // ... more UI colors
}
```

## Priority
Low - Visual enhancement, current theme works
