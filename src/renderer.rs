// Renderer + UI shell

use iced::widget::{Canvas, Column, Row, Text, Scrollable, Container, container, Checkbox};
use iced::widget::button::Button;
use iced::widget::text_input::TextInput;
use iced::{Element, Length, Color, Point, Size, Rectangle, Theme, Pixels, Font, Alignment, Border, Background};
use iced::widget::canvas::{self, Program, Frame};
use iced::mouse::Cursor;
use vt100;
use chrono::Utc;
use crate::{Message, AiSettings, Block, ThemeConfig, Tab, AiChatMessage, AiChatRole, AiContextScope, AiQuickAction};
use crate::export::ExportFormat;
use std::collections::HashMap;
use std::hash::{Hash, Hasher, DefaultHasher};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct StyleRun {
    text: String,
    fg: Color,
    bg: Color,
    x: f32,
    width: f32,
}

fn compute_row_hash(screen: &vt100::Screen, row: u16) -> u64 {
    let mut hasher = DefaultHasher::new();
    let cols = screen.size().0;
    for col in 0..cols {
        if let Some(cell) = screen.cell(row, col) {
            cell.contents().hash(&mut hasher);
            format!("{:?}", cell.fgcolor()).hash(&mut hasher);
            format!("{:?}", cell.bgcolor()).hash(&mut hasher);
        }
    }
    hasher.finish()
}

fn compute_runs(screen: &vt100::Screen, row: u16, cell_width: f32, _cell_height: f32) -> Vec<StyleRun> {
    let cols = screen.size().0;
    let mut runs = vec![];
    let mut col = 0;
    while col < cols {
        if let Some(cell) = screen.cell(row, col) {
            let start_col = col;
            let fg = color_to_iced(cell.fgcolor());
            let bg = bgcolor_to_iced(cell.bgcolor());
            let mut text = cell.contents().to_string();
            col += 1;
            while col < cols {
                if let Some(next_cell) = screen.cell(row, col) {
                    if color_to_iced(next_cell.fgcolor()) == fg && bgcolor_to_iced(next_cell.bgcolor()) == bg {
                        text.push_str(&next_cell.contents());
                        col += 1;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            let x = start_col as f32 * cell_width;
            let width = (col - start_col) as f32 * cell_width;
            runs.push(StyleRun { text, fg, bg, x, width });
        } else {
            col += 1;
        }
    }
    runs
}

fn draw_runs(frame: &mut Frame, runs: &[StyleRun], y: f32, cell_height: f32) {
    for run in runs {
        frame.fill_rectangle(Point::new(run.x, y), Size::new(run.width, cell_height), run.bg);
        if !run.text.is_empty() && run.text != " ".repeat(run.text.len()) {
            let text_canvas = canvas::Text {
                content: run.text.clone(),
                position: Point::new(run.x, y),
                size: Pixels(cell_height),
                color: run.fg,
                font: Font::MONOSPACE,
                ..canvas::Text::default()
            };
            frame.fill_text(text_canvas);
        }
    }
}

pub struct TerminalRenderer;

#[derive(Debug, Clone, Copy)]
struct MatchRanges {
    command: bool,
    output: bool,
}

fn normalize_query(query: &str) -> String {
    query.trim().to_lowercase()
}

fn block_matches_query(block: &Block, query: &str) -> MatchRanges {
    if query.is_empty() {
        return MatchRanges { command: false, output: false };
    }
    let q = normalize_query(query);
    let cmd_match = block.command.to_lowercase().contains(&q);
    let out_match = block.output.to_lowercase().contains(&q);
    MatchRanges { command: cmd_match, output: out_match }
}

fn matches_filters(block: &Block, query: &str, success_only: bool, failure_only: bool, pinned_only: bool) -> Option<MatchRanges> {
    if pinned_only && !block.pinned {
        return None;
    }
    if success_only && block.exit_code != Some(0) {
        return None;
    }
    if failure_only && matches!(block.exit_code, Some(0) | None) {
        return None;
    }
    if query.is_empty() {
        return Some(MatchRanges { command: false, output: false });
    }
    let ranges = block_matches_query(block, query);
    if ranges.command || ranges.output {
        Some(ranges)
    } else {
        None
    }
}

fn screen_to_text(screen: &vt100::Screen) -> String {
    let size = screen.size();
    let cols = size.0 as usize;
    let rows = size.1 as usize;
    let mut out = String::new();
    for row in 0..rows {
        let mut line = String::new();
        for col in 0..cols {
            if let Some(cell) = screen.cell(row as u16, col as u16) {
                line.push_str(&cell.contents());
            }
        }
        out.push_str(line.trim_end());
        if row + 1 < rows {
            out.push('\n');
        }
    }
    out
}

fn last_non_empty_line(text: &str) -> Option<String> {
    text.lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.to_string())
}

fn strip_prompt_line(output: &str, prompt_line: &str) -> String {
    if prompt_line.trim().is_empty() {
        return output.to_string();
    }
    let mut lines: Vec<&str> = output.lines().collect();
    while let Some(last) = lines.last() {
        if last.trim().is_empty() {
            lines.pop();
            continue;
        }
        if last.trim_end() == prompt_line.trim_end() {
            lines.pop();
        }
        break;
    }
    lines.join("\n")
}

fn color_to_iced(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Rgb(r, g, b) => Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0),
        vt100::Color::Idx(idx) => {
            // Standard ANSI colors
            match idx {
                0 => Color::from_rgb(0.0, 0.0, 0.0),       // Black
                1 => Color::from_rgb(0.8, 0.0, 0.0),       // Red
                2 => Color::from_rgb(0.0, 0.8, 0.0),       // Green
                3 => Color::from_rgb(0.8, 0.8, 0.0),       // Yellow
                4 => Color::from_rgb(0.0, 0.0, 0.8),       // Blue
                5 => Color::from_rgb(0.8, 0.0, 0.8),       // Magenta
                6 => Color::from_rgb(0.0, 0.8, 0.8),       // Cyan
                7 => Color::from_rgb(0.9, 0.9, 0.9),       // White
                8 => Color::from_rgb(0.5, 0.5, 0.5),       // Bright Black
                9 => Color::from_rgb(1.0, 0.0, 0.0),       // Bright Red
                10 => Color::from_rgb(0.0, 1.0, 0.0),      // Bright Green
                11 => Color::from_rgb(1.0, 1.0, 0.0),      // Bright Yellow
                12 => Color::from_rgb(0.0, 0.0, 1.0),      // Bright Blue
                13 => Color::from_rgb(1.0, 0.0, 1.0),      // Bright Magenta
                14 => Color::from_rgb(0.0, 1.0, 1.0),      // Bright Cyan
                15 => Color::from_rgb(1.0, 1.0, 1.0),      // Bright White
                _ => Color::from_rgb(0.9, 0.9, 0.9),       // Default to white
            }
        }
        vt100::Color::Default => Color::from_rgb(0.9, 0.9, 0.9), // Default foreground
    }
}

fn bgcolor_to_iced(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => default_bg_color(), // Use ash gray for default background
        _ => color_to_iced(color), // Use regular color mapping for explicit colors
    }
}

fn default_bg_color() -> Color {
    Color::from_rgb(0.15, 0.15, 0.15) // Dark ash gray background
}

impl TerminalRenderer {
    pub fn new() -> Self {
        TerminalRenderer
    }

    pub fn cell_size(&self, theme_config: &ThemeConfig) -> (f32, f32) {
        (8.0, theme_config.line_height * theme_config.font_size)
    }

    pub fn view<'a>(&self, history: &'a [Block], current: &'a Option<Block>, current_command: &'a str, search_query: &'a str, search_success_only: bool, search_failure_only: bool, search_pinned_only: bool, search_input_id: iced::widget::text_input::Id, screen: &vt100::Screen, alt_screen_active: bool, _ai_settings: &'a AiSettings, _ai_response: &'a Option<String>, _scroll_offset: usize, _selection_start: Option<(usize, usize)>, _selection_end: Option<(usize, usize)>, render_cache: &Arc<Mutex<HashMap<(usize, usize, u16), Vec<StyleRun>>>>, row_hashes: &Arc<Mutex<HashMap<(usize, usize, u16), u64>>>, tab_id: usize, pane_id: usize, theme_config: &'a ThemeConfig, tabs: &'a [Tab], active_tab: usize, renaming_tab: Option<usize>, rename_buffer: &'a str, history_search_active: bool, history_search_query: &'a str, history_matches: &'a [String], history_selected: usize, ai_panel_open: bool, ai_context_scope: AiContextScope, ai_chat: &'a [AiChatMessage], ai_input: &'a str, ai_pending: bool, ai_streaming: bool) -> Element<'a, Message> {
        // Use raw terminal mode for TUI apps (vim, top, etc.), block mode for normal shell
        if alt_screen_active {
            let canvas = Canvas::new(TerminalCanvas {
                screen: screen.clone(),
                cell_width: 8.0,
                cell_height: theme_config.line_height * theme_config.font_size,
                render_cache: render_cache.clone(),
                row_hashes: row_hashes.clone(),
                tab_id,
                pane_id,
            })
            .width(Length::Fill)
            .height(Length::Fill);

            if ai_panel_open {
                let panel = self.render_ai_panel(ai_context_scope, ai_chat, ai_input, ai_pending, ai_streaming, pane_id, theme_config);
                Row::new()
                    .push(Container::new(canvas).width(Length::FillPortion(7)))
                    .push(Container::new(panel).width(Length::FillPortion(3)))
                    .height(Length::Fill)
                    .into()
            } else {
                canvas.into()
            }
        } else {
            self.render_blocks(history, current, current_command, search_query, search_success_only, search_failure_only, search_pinned_only, search_input_id, screen, theme_config, tabs, active_tab, renaming_tab, rename_buffer, history_search_active, history_search_query, history_matches, history_selected, ai_panel_open, ai_context_scope, ai_chat, ai_input, ai_pending, ai_streaming, pane_id)
        }
    }

    fn render_blocks<'a>(&self, history: &'a [Block], current: &'a Option<Block>, current_command: &'a str, search_query: &'a str, search_success_only: bool, search_failure_only: bool, search_pinned_only: bool, search_input_id: iced::widget::text_input::Id, screen: &vt100::Screen, theme_config: &'a ThemeConfig, tabs: &'a [Tab], active_tab: usize, renaming_tab: Option<usize>, rename_buffer: &'a str, history_search_active: bool, history_search_query: &'a str, history_matches: &'a [String], history_selected: usize, ai_panel_open: bool, ai_context_scope: AiContextScope, ai_chat: &'a [AiChatMessage], ai_input: &'a str, ai_pending: bool, ai_streaming: bool, pane_id: usize) -> Element<'a, Message> {
        let mut column = Column::new().spacing(10).padding(theme_config.padding as u16);

        let live_screen_text = screen_to_text(screen);
        let prompt_line = last_non_empty_line(&live_screen_text).unwrap_or_default();

        let mut match_count = 0usize;
        let mut filtered_blocks: Vec<(usize, &Block, MatchRanges)> = vec![];
        for (index, block) in history.iter().enumerate() {
            if let Some(ranges) = matches_filters(block, search_query, search_success_only, search_failure_only, search_pinned_only) {
                if !search_query.trim().is_empty() {
                    if ranges.command { match_count += 1; }
                    if ranges.output { match_count += 1; }
                }
                filtered_blocks.push((index, block, ranges));
            }
        }

        let search_input = TextInput::new("Search commands/output...", search_query)
            .id(search_input_id)
            .on_input(Message::UpdateSearch)
            .size(12.0)
            .padding(6)
            .font(Font::MONOSPACE);

        let search_badge = Text::new(format!("Matches: {}", match_count))
            .size(11.0)
            .style(Color::from_rgb(0.7, 0.7, 0.7));

        let success_filter = Button::new(Text::new("Success").size(11.0))
            .on_press(Message::ToggleSearchSuccess);
        let failure_filter = Button::new(Text::new("Failure").size(11.0))
            .on_press(Message::ToggleSearchFailure);
        let pinned_filter = Button::new(Text::new("Pinned").size(11.0))
            .on_press(Message::ToggleSearchPinned);
        let clear_filter = Button::new(Text::new("Clear").size(11.0))
            .on_press(Message::ClearSearch);

        let select_all = Button::new(Text::new("Select all").size(11.0))
            .on_press(Message::SelectAllBlocks);
        let deselect_all = Button::new(Text::new("Deselect all").size(11.0))
            .on_press(Message::DeselectAllBlocks);
        let export_selected_md = Button::new(Text::new("Export MD").size(11.0))
            .on_press(Message::ExportSelected(ExportFormat::Markdown));
        let export_selected_json = Button::new(Text::new("Export JSON").size(11.0))
            .on_press(Message::ExportSelected(ExportFormat::Json));
        let export_selected_html = Button::new(Text::new("Export HTML").size(11.0))
            .on_press(Message::ExportSelected(ExportFormat::Html));
        let export_selected_text = Button::new(Text::new("Export TXT").size(11.0))
            .on_press(Message::ExportSelected(ExportFormat::Text));

        let search_row = Row::new()
            .push(Container::new(search_input).width(Length::Fill))
            .push(search_badge)
            .push(success_filter)
            .push(failure_filter)
            .push(pinned_filter)
            .push(clear_filter)
            .push(select_all)
            .push(deselect_all)
            .push(export_selected_md)
            .push(export_selected_json)
            .push(export_selected_html)
            .push(export_selected_text)
            .spacing(8)
            .align_items(Alignment::Center);

        let tab_bar = self.render_tab_bar(tabs, active_tab, renaming_tab, rename_buffer, theme_config);
        column = column.push(tab_bar).push(search_row);

        // Show live screen text if no history yet (so prompts are visible)
        if history.is_empty() && current.is_none() {
            if live_screen_text.trim().is_empty() {
                let welcome = Text::new("Welcome to Tant Terminal\n\nType a command below and press Enter to get started.")
                    .size(14.0);
                column = column.push(welcome);
            } else {
                let live_output = Text::new(live_screen_text.clone())
                    .font(Font::MONOSPACE)
                    .size(theme_config.font_size - 2.0);
                column = column.push(live_output);
            }
        }

        // Render history blocks
        for (index, block, ranges) in filtered_blocks {
            let block_widget = self.render_block(block, index, theme_config, search_query, ranges, &prompt_line);
            column = column.push(block_widget);
        }

        // Render current block if running
        if let Some(block) = current {
            let current_block_widget = self.render_current_block(block, screen, theme_config);
            column = column.push(current_block_widget);
        }

        let scrollable = Scrollable::new(column)
            .width(Length::Fill)
            .height(Length::FillPortion(9));

        let history_panel = if history_search_active {
            let mut list = Column::new().spacing(4);
            for (index, item) in history_matches.iter().enumerate() {
                let color = if index == history_selected {
                    Color::from_rgb(1.0, 0.85, 0.4)
                } else {
                    Color::from_rgb(0.8, 0.8, 0.8)
                };
                list = list.push(
                    Text::new(item.clone())
                        .font(Font::MONOSPACE)
                        .size(theme_config.font_size - 2.0)
                        .style(color),
                );
            }
            let header = Text::new(format!("Reverse search: {}", history_search_query))
                .font(Font::MONOSPACE)
                .size(theme_config.font_size - 2.0)
                .style(Color::from_rgb(0.7, 0.7, 0.7));
            Some(
                Container::new(Column::new().spacing(6).push(header).push(list))
                    .padding(8)
                    .style(|_theme: &Theme| container::Appearance {
                        background: Some(Background::Color(Color::from_rgb(0.12, 0.12, 0.12))),
                        border: Border {
                            radius: 6.0.into(),
                            width: 1.0,
                            color: Color::from_rgb(0.25, 0.25, 0.25),
                        },
                        ..Default::default()
                    }),
            )
        } else {
            None
        };

        // Command input area with better styling and increased height
        let input = TextInput::new("Type a command here...", current_command)
            .on_input(Message::TerminalInput)
            .on_submit(Message::TerminalSubmit)
            .padding(18)
            .size(theme_config.font_size)
            .font(Font::MONOSPACE); // TODO: use theme_config.font_family
        
        // Wrap input in a highly visible container
        let input_with_bg = Container::new(input)
            .width(Length::Fill)
            .padding(4)
            .style(|_theme: &Theme| container::Appearance {
                background: Some(Background::Color(Color::from_rgb(0.35, 0.35, 0.35))),
                border: Border {
                    color: Color::from_rgb(0.5, 0.7, 1.0),  // Bright blue border
                    width: 2.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            });

        // Create metadata labels row - always show with fallback values
        let mut metadata_row = Row::new().spacing(15);
        
        let block_for_metadata = current.as_ref().or_else(|| history.last());
        
        // Get metadata from block or use fallbacks
        let (cwd_str, git_branch, git_status, host, is_remote) = if let Some(block) = block_for_metadata {
            (
                block.cwd.as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "~".to_string()),
                block.git_branch.clone(),
                block.git_status.clone(),
                block.host.clone(),
                block.is_remote,
            )
        } else {
            // Fallback values when no blocks exist
            (
                std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "~".to_string()),
                None,
                None,
                "localhost".to_string(),
                false,
            )
        };
        
        // Directory label
        let dir_label = Row::new()
            .push(Text::new("📁 ").size(10.0))
            .push(Text::new(cwd_str).size(10.0).font(Font::MONOSPACE))
            .spacing(3);
        metadata_row = metadata_row.push(dir_label);
        
        // Git branch label (if available)
        if let Some(branch) = git_branch {
            let branch_color = match branch.as_str() {
                "main" | "master" => Color::from_rgb(0.2, 0.75, 0.45),
                _ => Color::from_rgb(0.55, 0.65, 0.95),
            };
            let status_indicator = match git_status {
                Some(crate::parser::GitStatus::Clean) => "✔",
                Some(crate::parser::GitStatus::Dirty) => "●",
                Some(crate::parser::GitStatus::Conflicts) => "✖",
                None => "",
            };
            let status_color = match git_status {
                Some(crate::parser::GitStatus::Clean) => Color::from_rgb(0.2, 0.75, 0.45),
                Some(crate::parser::GitStatus::Dirty) => Color::from_rgb(0.9, 0.55, 0.2),
                Some(crate::parser::GitStatus::Conflicts) => Color::from_rgb(0.9, 0.35, 0.35),
                None => Color::from_rgb(0.65, 0.65, 0.65),
            };
            let branch_label = Row::new()
                .push(Text::new("🌿 ").size(10.0))
                .push(Text::new(branch).size(10.0).font(Font::MONOSPACE).style(branch_color))
                .spacing(3);
            let branch_with_status = if status_indicator.is_empty() {
                branch_label
            } else {
                branch_label
                    .push(Text::new(format!(" {}", status_indicator)).size(10.0).style(status_color))
            };
            metadata_row = metadata_row.push(branch_with_status);
        }
        
        // Host label
        let host_icon = if is_remote { "🛰️ " } else { "💻 " };
        let host_color = if is_remote {
            Color::from_rgb(0.85, 0.55, 0.2)
        } else {
            Color::from_rgb(0.7, 0.7, 0.7)
        };
        let host_label = Row::new()
            .push(Text::new(host_icon).size(10.0))
            .push(Text::new(host).size(10.0).font(Font::MONOSPACE).style(host_color))
            .spacing(3);
        metadata_row = metadata_row.push(host_label);

        let prompt_color = match current.as_ref().and_then(|block| block.exit_code) {
            Some(0) => Color::from_rgb(0.2, 0.8, 0.4),
            Some(_) => Color::from_rgb(0.9, 0.35, 0.35),
            None => Color::from_rgb(0.6, 0.8, 1.0),
        };
        let prompt_text = if prompt_line.trim().is_empty() {
            "❯".to_string()
        } else {
            prompt_line.to_string()
        };
        let prompt_row = Container::new(
            Text::new(prompt_text)
                .font(Font::MONOSPACE)
                .size(theme_config.font_size)
                .style(prompt_color),
        )
        .padding([6, 12, 0, 12]);

        // Input area with labels
        let mut input_area = Column::new()
            .push(prompt_row)
            .push(input_with_bg)
            .spacing(0);
        if let Some(panel) = history_panel {
            input_area = input_area.push(panel);
        }
        input_area = input_area.push(Container::new(metadata_row).padding([5, 12, 8, 12]));

        let input_container = Container::new(input_area)
            .width(Length::Fill)
            .height(Length::FillPortion(1))
            .padding(0);

        let terminal_column = Column::new()
            .push(scrollable)
            .push(input_container)
            .height(Length::Fill)
            .spacing(0);

        let main_area: Element<'a, Message> = if ai_panel_open {
            let panel = self.render_ai_panel(ai_context_scope, ai_chat, ai_input, ai_pending, ai_streaming, pane_id, theme_config);
            Row::new()
                .push(Container::new(terminal_column).width(Length::FillPortion(7)))
                .push(Container::new(panel).width(Length::FillPortion(3)))
                .height(Length::Fill)
                .into()
        } else {
            terminal_column.into()
        };

        main_area
    }

    fn render_ai_panel<'a>(&self, ai_context_scope: AiContextScope, ai_chat: &'a [AiChatMessage], ai_input: &'a str, ai_pending: bool, ai_streaming: bool, pane_id: usize, theme_config: &'a ThemeConfig) -> Element<'a, Message> {
        let mut header = Row::new().spacing(8).align_items(Alignment::Center);
        header = header.push(Text::new("AI Assistant").size(14.0));

        let scope_label = match ai_context_scope {
            AiContextScope::CurrentCommand => "Context: Current command",
            AiContextScope::LastNBlocks => "Context: Last N blocks",
            AiContextScope::SelectedBlocks => "Context: Selected blocks",
            AiContextScope::EntireSession => "Context: Entire session",
        };
        let scope_text = Text::new(scope_label).size(11.0).style(Color::from_rgb(0.7, 0.7, 0.7));

        let scope_row = Row::new()
            .push(Button::new(Text::new("Current").size(11.0)).on_press(Message::AiPanelSetScope(pane_id, AiContextScope::CurrentCommand)))
            .push(Button::new(Text::new("Last N").size(11.0)).on_press(Message::AiPanelSetScope(pane_id, AiContextScope::LastNBlocks)))
            .push(Button::new(Text::new("Selected").size(11.0)).on_press(Message::AiPanelSetScope(pane_id, AiContextScope::SelectedBlocks)))
            .push(Button::new(Text::new("All").size(11.0)).on_press(Message::AiPanelSetScope(pane_id, AiContextScope::EntireSession)))
            .spacing(6);

        let quick_actions = Row::new()
            .push(Button::new(Text::new("Explain Error").size(11.0)).on_press(Message::AiPanelQuickAction(pane_id, AiQuickAction::ExplainError)))
            .push(Button::new(Text::new("Summarize Output").size(11.0)).on_press(Message::AiPanelQuickAction(pane_id, AiQuickAction::SummarizeOutput)))
            .push(Button::new(Text::new("Generate Command").size(11.0)).on_press(Message::AiPanelQuickAction(pane_id, AiQuickAction::GenerateCommand)))
            .spacing(6);

        let mut chat_column = Column::new().spacing(8);
        for message in ai_chat {
            let role_label = match message.role {
                AiChatRole::User => "You",
                AiChatRole::Assistant => "AI",
            };
            let role_color = match message.role {
                AiChatRole::User => Color::from_rgb(0.55, 0.75, 0.95),
                AiChatRole::Assistant => Color::from_rgb(0.75, 0.85, 0.65),
            };
            let header = Row::new()
                .push(Text::new(role_label).size(11.0).style(role_color))
                .spacing(4);

            let sources = if message.sources.is_empty() {
                Text::new("sources: none").size(10.0).style(Color::from_rgb(0.55, 0.55, 0.55))
            } else {
                Text::new(format!("sources: {}", message.sources.join(", ")))
                    .size(10.0)
                    .style(Color::from_rgb(0.55, 0.55, 0.55))
            };
            let body = Text::new(message.content.clone())
                .font(Font::MONOSPACE)
                .size(theme_config.font_size - 2.0);
            let bubble = Column::new().spacing(4).push(header).push(sources).push(body);
            let bubble_container = Container::new(bubble)
                .padding(8)
                .style(|_theme: &Theme| container::Appearance {
                    background: Some(Background::Color(Color::from_rgb(0.16, 0.16, 0.18))),
                    border: Border {
                        radius: 6.0.into(),
                        width: 1.0,
                        color: Color::from_rgb(0.24, 0.24, 0.24),
                    },
                    ..Default::default()
                });
            chat_column = chat_column.push(bubble_container);
        }

        let chat_scroll = Scrollable::new(chat_column)
            .height(Length::Fill)
            .width(Length::Fill);

        let input = TextInput::new("Ask anything...", ai_input)
            .on_input(move |value| Message::AiPanelInputChanged(pane_id, value))
            .on_submit(Message::AiPanelSend(pane_id))
            .padding(8)
            .size(theme_config.font_size - 1.0)
            .font(Font::MONOSPACE);

        let send_button = Button::new(Text::new("Send").size(12.0))
            .on_press(Message::AiPanelSend(pane_id));
        let stop_button = Button::new(Text::new("Stop").size(12.0))
            .on_press(Message::AiPanelStop(pane_id));

        let status_text = if ai_pending || ai_streaming {
            Text::new("Generating...").size(11.0).style(Color::from_rgb(0.75, 0.75, 0.75))
        } else {
            Text::new("Idle").size(11.0).style(Color::from_rgb(0.55, 0.55, 0.55))
        };

        let input_row = Row::new()
            .push(Container::new(input).width(Length::Fill))
            .push(send_button)
            .push(stop_button)
            .spacing(6)
            .align_items(Alignment::Center);

        let body = Column::new()
            .spacing(10)
            .push(header)
            .push(scope_text)
            .push(scope_row)
            .push(quick_actions)
            .push(chat_scroll)
            .push(input_row)
            .push(status_text)
            .padding(10);

        Container::new(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme: &Theme| container::Appearance {
                background: Some(Background::Color(Color::from_rgb(0.12, 0.13, 0.16))),
                border: Border {
                    radius: 6.0.into(),
                    width: 1.0,
                    color: Color::from_rgb(0.22, 0.22, 0.22),
                },
                ..Default::default()
            })
            .into()
    }

    fn render_tab_bar<'a>(&self, tabs: &'a [Tab], active_tab: usize, renaming_tab: Option<usize>, rename_buffer: &'a str, _theme_config: &'a ThemeConfig) -> Element<'a, Message> {
        let mut row = Row::new().spacing(6).align_items(Alignment::Center);
        for (index, tab) in tabs.iter().enumerate() {
            let is_active = index == active_tab;
            let title_widget: Element<'a, Message> = if renaming_tab == Some(index) {
                TextInput::new("Tab name", rename_buffer)
                    .on_input(Message::RenameTabInput)
                    .on_submit(Message::CommitRenameTab)
                    .size(12.0)
                    .padding(6)
                    .into()
            } else {
                let title_text = Text::new(tab.title.clone())
                    .font(Font::MONOSPACE)
                    .size(12.0)
                    .style(if is_active { Color::WHITE } else { Color::from_rgb(0.7, 0.7, 0.7) });
                Button::new(title_text)
                    .on_press(Message::SelectTab(index))
                    .padding([4, 8])
                    .into()
            };

            let close_button = Button::new(Text::new("×").size(12.0))
                .on_press(Message::CloseTabAt(index))
                .padding([2, 6]);

            let tab_chip = Row::new()
                .push(title_widget)
                .push(close_button)
                .spacing(4)
                .align_items(Alignment::Center);

            let tab_container = Container::new(tab_chip)
                .padding([4, 6])
                .style(move |_theme: &Theme| container::Appearance {
                    background: Some(Background::Color(if is_active { Color::from_rgb(0.2, 0.2, 0.2) } else { Color::from_rgb(0.12, 0.12, 0.12) })),
                    border: Border {
                        radius: 6.0.into(),
                        width: 1.0,
                        color: if is_active { Color::from_rgb(0.45, 0.75, 1.0) } else { Color::from_rgb(0.2, 0.2, 0.2) },
                    },
                    ..Default::default()
                });
            row = row.push(tab_container);
        }

        row = row.push(
            Button::new(Text::new("+").size(14.0))
                .on_press(Message::NewTab)
                .padding([4, 8]),
        );

        Container::new(row)
            .padding([6, 8])
            .style(|_theme: &Theme| container::Appearance {
                background: Some(Background::Color(Color::from_rgb(0.1, 0.1, 0.1))),
                border: Border {
                    radius: 8.0.into(),
                    width: 1.0,
                    color: Color::from_rgb(0.18, 0.18, 0.18),
                },
                ..Default::default()
            })
            .width(Length::Fill)
            .into()
    }

    fn render_block<'a>(&self, block: &'a Block, index: usize, theme_config: &'a ThemeConfig, search_query: &'a str, ranges: MatchRanges, prompt_line: &str) -> Element<'a, Message> {
        let (status_display, status_color) = match block.exit_code {
            Some(0) => ("Success".to_string(), Color::from_rgb(0.25, 0.8, 0.4)),
            Some(code) => (format!("Exit {}", code), Color::from_rgb(0.9, 0.35, 0.35)),
            None => ("Running".to_string(), Color::from_rgb(0.6, 0.6, 0.6)),
        };

        let duration_text = block.duration_ms
            .map(|ms| format!("{:.2}s", ms as f64 / 1000.0))
            .unwrap_or_else(|| "...".to_string());

        // Command line with prompt symbol
        let prompt = Text::new("❯")
            .font(Font::MONOSPACE)
            .size(theme_config.font_size + 2.0)
            .style(Color::from_rgb(0.6, 0.8, 1.0));

        let command_color = if ranges.command && !search_query.trim().is_empty() {
            Color::from_rgb(1.0, 0.85, 0.4)
        } else {
            Color::WHITE
        };
        let command = Text::new(&block.command)
            .font(Font::MONOSPACE)
            .size(theme_config.font_size)
            .style(command_color);

        let status = Container::new(
            Text::new(status_display)
                .size(11.0)
                .style(Color::WHITE),
        )
        .padding([2, 8])
        .style(move |_theme: &Theme| container::Appearance {
            background: Some(Background::Color(status_color)),
            border: Border {
                radius: 12.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            ..Default::default()
        });

        let duration = Text::new(duration_text)
            .size(11.0)
            .style(Color::from_rgb(0.75, 0.75, 0.75));

        let select_checkbox = Checkbox::new("", block.selected)
            .on_toggle(move |_| Message::ToggleBlockSelected(index));
        let header = Row::new()
            .push(select_checkbox)
            .push(prompt)
            .push(command)
            .spacing(8)
            .align_items(Alignment::Center);

        let meta_row = Row::new()
            .push(status)
            .push(duration)
            .spacing(8)
            .align_items(Alignment::Center);

        let buttons = Row::new()
            .push(Button::new(Text::new("Copy").size(11.0)).on_press(Message::CopyCommand(index)))
            .push(Button::new(Text::new("Copy Output").size(11.0)).on_press(Message::CopyOutput(index)))
            .push(Button::new(Text::new("Export MD").size(11.0)).on_press(Message::ExportBlock(index, ExportFormat::Markdown)))
            .push(Button::new(Text::new("Export JSON").size(11.0)).on_press(Message::ExportBlock(index, ExportFormat::Json)))
            .push(Button::new(Text::new("Export HTML").size(11.0)).on_press(Message::ExportBlock(index, ExportFormat::Html)))
            .push(Button::new(Text::new("Export TXT").size(11.0)).on_press(Message::ExportBlock(index, ExportFormat::Text)))
            .push(Button::new(Text::new("Rerun").size(11.0)).on_press(Message::RerunCommand(index)))
            .push(Button::new(Text::new(if block.collapsed { "Show" } else { "Hide" }).size(11.0)).on_press(Message::ToggleCollapsed(index)))
            .push(Button::new(Text::new(if block.pinned { "📌" } else { "Pin" }).size(11.0)).on_press(Message::TogglePin(index)))
            .spacing(6);

        let mut column = Column::new()
            .push(Row::new()
                .push(Container::new(header).width(Length::Fill))
                .push(buttons)
                .align_items(Alignment::Center)
                .spacing(10))
            .push(meta_row)
            .spacing(6)
            .padding([10, 12]);

        if !block.collapsed && !block.output.is_empty() {
            let output = strip_prompt_line(&block.output, prompt_line);
            let output_color = if ranges.output && !search_query.trim().is_empty() {
                Color::from_rgb(1.0, 0.9, 0.55)
            } else {
                Color::from_rgb(0.85, 0.85, 0.85)
            };
            let output_text = Text::new(output)
                .font(Font::MONOSPACE)
                .size(theme_config.font_size - 3.0)
                .style(output_color);
            let output_container = Container::new(output_text)
                .padding(8)
                .style(|_theme: &Theme| container::Appearance {
                    background: Some(Background::Color(Color::from_rgb(0.18, 0.18, 0.18))),
                    border: Border {
                        radius: 6.0.into(),
                        width: 1.0,
                        color: Color::from_rgb(0.25, 0.25, 0.25),
                    },
                    ..Default::default()
                });
            column = column.push(output_container);
        }

        Container::new(column)
            .width(Length::Fill)
            .padding(6)
            .style(|_theme: &Theme| container::Appearance {
                background: Some(Background::Color(Color::from_rgb(0.13, 0.13, 0.13))),
                border: Border {
                    radius: 10.0.into(),
                    width: 1.0,
                    color: Color::from_rgb(0.2, 0.2, 0.2),
                },
                ..Default::default()
            })
            .into()
    }

    fn render_current_block<'a>(&self, block: &'a Block, screen: &vt100::Screen, theme_config: &'a ThemeConfig) -> Element<'a, Message> {
        let duration_text = block.started_at
            .map(|start| format!("{:.2}s", (Utc::now() - start).num_milliseconds() as f64 / 1000.0))
            .unwrap_or_else(|| "...".to_string());

        // Command line with prompt symbol
        let prompt = Text::new("❯")
            .font(Font::MONOSPACE)
            .size(theme_config.font_size + 2.0)
            .style(Color::from_rgb(0.6, 0.8, 1.0));

        let command = Text::new(&block.command)
            .font(Font::MONOSPACE)
            .size(theme_config.font_size);

        let status = Container::new(
            Text::new("Running")
                .size(11.0)
                .style(Color::WHITE),
        )
        .padding([2, 8])
        .style(|_theme: &Theme| container::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.4, 0.6, 0.9))),
            border: Border {
                radius: 12.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            ..Default::default()
        });

        let duration = Text::new(duration_text)
            .size(11.0)
            .style(Color::from_rgb(0.75, 0.75, 0.75));

        let header = Row::new()
            .push(prompt)
            .push(command)
            .spacing(8)
            .align_items(Alignment::Center);

        let meta_row = Row::new()
            .push(status)
            .push(duration)
            .spacing(8)
            .align_items(Alignment::Center);

        let mut column = Column::new()
            .push(header)
            .push(meta_row)
            .spacing(6)
            .padding([10, 12]);

        let live_output_text = if !block.output.is_empty() {
            block.output.clone()
        } else {
            screen_to_text(screen)
        };

        if !live_output_text.trim().is_empty() {
            let output_text = Text::new(live_output_text)
                .font(Font::MONOSPACE)
                .size(theme_config.font_size - 3.0);
            let output_container = Container::new(output_text)
                .padding(8)
                .style(|_theme: &Theme| container::Appearance {
                    background: Some(Background::Color(Color::from_rgb(0.18, 0.18, 0.18))),
                    border: Border {
                        radius: 6.0.into(),
                        width: 1.0,
                        color: Color::from_rgb(0.25, 0.25, 0.25),
                    },
                    ..Default::default()
                });
            column = column.push(output_container);
        }

        Container::new(column)
            .width(Length::Fill)
            .padding(6)
            .style(|_theme: &Theme| container::Appearance {
                background: Some(Background::Color(Color::from_rgb(0.13, 0.13, 0.13))),
                border: Border {
                    radius: 10.0.into(),
                    width: 1.0,
                    color: Color::from_rgb(0.2, 0.2, 0.2),
                },
                ..Default::default()
            })
            .into()
    }
}

pub struct TerminalCanvas {
    pub screen: vt100::Screen,
    pub cell_width: f32,
    pub cell_height: f32,
    pub render_cache: Arc<Mutex<HashMap<(usize, usize, u16), Vec<StyleRun>>>>,
    pub row_hashes: Arc<Mutex<HashMap<(usize, usize, u16), u64>>>,
    pub tab_id: usize,
    pub pane_id: usize,
}

impl Program<Message> for TerminalCanvas {
    type State = ();

    fn draw(&self, _state: &Self::State, renderer: &iced::Renderer, _theme: &Theme, bounds: Rectangle, _cursor: Cursor) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        // Fill with default background
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), default_bg_color());

        let size = self.screen.size();
        let rows = size.1 as usize;

        let mut cache = self.render_cache.lock().unwrap();
        let mut hashes = self.row_hashes.lock().unwrap();
        for row in 0..rows {
            let y = row as f32 * self.cell_height;
            let key = (self.tab_id, self.pane_id, row as u16);
            let hash = compute_row_hash(&self.screen, row as u16);
            if hashes.get(&key) != Some(&hash) {
                let runs = compute_runs(&self.screen, row as u16, self.cell_width, self.cell_height);
                cache.insert(key, runs.clone());
                hashes.insert(key, hash);
                draw_runs(&mut frame, &runs, y, self.cell_height);
            } else {
                if let Some(runs) = cache.get(&key) {
                    draw_runs(&mut frame, runs, y, self.cell_height);
                }
            }
        }

        vec![frame.into_geometry()]
    }
}
