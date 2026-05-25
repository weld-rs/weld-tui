use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Paragraph};

use weld_core::file::diff::{BlockKind, DiffResult};
use weld_core::file::display::DisplayRow;
use weld_core::file::inline_diff::InlineKind;
use weld_core::text::expand_tabs;

use crate::app::App;
use crate::overlay::{self, Overlay};
use crate::theme::Theme;

/// Fixed width (in columns) of the minimap pane when shown.
const MINIMAP_WIDTH: u16 = 1;

/// Gutter + code lines for one side of the diff.
struct SideLines {
    gutter: Vec<ratatui::text::Line<'static>>,
    code: Vec<ratatui::text::Line<'static>>,
}

use crate::app::Side;

/// Shared parameters for rendering a file pane.
struct PaneContext<'a> {
    dir: &'a str,
    filename: &'a str,
    lines: &'a [String],
    display_rows: &'a [DisplayRow],
    diff: &'a DiffResult,
    side: Side,
    scroll_y: u16,
    scroll_x: u16,
    digit_width: usize,
    max_content_width: usize,
    active_block_index: Option<usize>,
    dirty: bool,
    theme: &'a Theme,
    tab_width: usize,
}

fn build_side_lines(ctx: &PaneContext, gutter_width: u16) -> SideLines {
    let display_rows = ctx.display_rows;
    let lines = ctx.lines;
    let side = ctx.side;
    let digit_width = ctx.digit_width;
    let max_content_width = ctx.max_content_width;
    let diff = ctx.diff;
    let theme = ctx.theme;
    let active_block_index = ctx.active_block_index;
    let tab_width = ctx.tab_width;

    let mut gutter = Vec::with_capacity(display_rows.len());
    let mut code = Vec::with_capacity(display_rows.len());

    for row in display_rows {
        let line_idx = match side {
            Side::Left => row.left_line,
            Side::Right => row.right_line,
        };

        let is_diff = row.kind != BlockKind::Equal;
        let is_active = active_block_index == Some(row.block_index);
        let bg = if !is_diff {
            theme.bg
        } else if is_active {
            theme.diff_bg_active
        } else {
            theme.diff_bg
        };

        // Gutter always uses gutter_bg
        let gutter_style = Style::default()
            .fg(theme.line_number_fg)
            .bg(theme.gutter_bg);

        if let Some(idx) = line_idx {
            gutter.push(ratatui::text::Line::from(Span::styled(
                format!(" {:>width$} ", idx + 1, width = digit_width),
                gutter_style,
            )));
        } else {
            gutter.push(ratatui::text::Line::from(Span::styled(
                " ".repeat(gutter_width as usize),
                gutter_style,
            )));
        }

        // Code — for Replace rows with inline diffs, highlight changed characters.
        if row.kind == BlockKind::Replace
            && let Some(inline) = inline_diff_for_row(row, side, diff)
        {
            let base_style = Style::default().fg(theme.fg).bg(bg);
            let emphasis_bg = if is_active {
                theme.diff_emphasis_bg_active
            } else {
                theme.diff_emphasis_bg
            };
            let emphasis_style = Style::default().fg(theme.fg).bg(emphasis_bg);

            let segments = match side {
                Side::Left => &inline.left_segments,
                Side::Right => &inline.right_segments,
            };

            let mut spans: Vec<Span<'static>> = Vec::new();
            spans.push(Span::styled(" ".to_string(), base_style)); // leading space

            for seg in segments {
                let text = expand_tabs(&seg.text, tab_width);
                let style = match seg.kind {
                    InlineKind::Equal => base_style,
                    InlineKind::Changed => emphasis_style,
                };
                spans.push(Span::styled(text, style));
            }

            // Pad to max width for uniform highlight block.
            let current_width: usize = spans.iter().map(|s| s.content.len()).sum();
            if current_width < max_content_width {
                spans.push(Span::styled(
                    " ".repeat(max_content_width - current_width),
                    base_style,
                ));
            }

            code.push(ratatui::text::Line::from(spans));
            continue;
        }

        // Default: uniform style for the whole line.
        let line_style = Style::default().fg(theme.fg).bg(bg);
        let text = if let Some(idx) = line_idx {
            format!(" {}", expand_tabs(&lines[idx], tab_width))
        } else {
            " ".to_string()
        };
        let padded = if is_diff {
            format!("{:<width$}", text, width = max_content_width)
        } else {
            text
        };
        code.push(ratatui::text::Line::from(padded).style(line_style));
    }

    SideLines { gutter, code }
}

/// Look up the InlineDiff for a Replace row, if one exists.
fn inline_diff_for_row<'a>(
    row: &DisplayRow,
    side: Side,
    diff: &'a DiffResult,
) -> Option<&'a weld_core::file::inline_diff::InlineDiff> {
    let block = &diff.blocks[row.block_index];
    // Compute offset of this row within its block.
    let offset = match side {
        Side::Left => {
            let line = row.left_line?;
            line.checked_sub(block.left_range.start)?
        }
        Side::Right => {
            let line = row.right_line?;
            line.checked_sub(block.right_range.start)?
        }
    };
    block.inline_diffs.get(offset)
}

/// Render a file side using display rows.
fn render_file_pane(frame: &mut Frame, area: ratatui::layout::Rect, ctx: &PaneContext) {
    let theme = ctx.theme;
    let border_style = Style::default().fg(theme.gutter_bg);

    let [header_area, content_area] =
        Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(area);

    // Header
    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(
            format!(" {} ", ctx.dir),
            Style::default().fg(theme.status_bar_fg),
        ))
        .style(Style::default().bg(theme.bg));
    let mut header_spans = vec![Span::styled(
        format!(" {}", ctx.filename),
        Style::default()
            .fg(theme.header_fg)
            .add_modifier(ratatui::style::Modifier::BOLD),
    )];
    if ctx.dirty {
        header_spans.push(Span::styled(
            " ●",
            Style::default().fg(theme.dirty_indicator),
        ));
    }
    frame.render_widget(
        Paragraph::new(ratatui::text::Line::from(header_spans)).block(header_block),
        header_area,
    );

    // Content
    let content_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(theme.bg));
    let inner = content_block.inner(content_area);
    frame.render_widget(content_block, content_area);

    let gutter_width = (ctx.digit_width + 2) as u16;
    let [gutter_area, code_area] =
        Layout::horizontal([Constraint::Length(gutter_width), Constraint::Min(0)]).areas(inner);

    let side_lines = build_side_lines(ctx, gutter_width);

    frame.render_widget(
        Paragraph::new(side_lines.gutter).scroll((ctx.scroll_y, 0)),
        gutter_area,
    );
    frame.render_widget(
        Paragraph::new(side_lines.code).scroll((ctx.scroll_y, ctx.scroll_x)),
        code_area,
    );
}

/// Build the status-bar hint line with optional chord-progress highlighting.
fn status_hint<'a>(app: &App, theme: &Theme) -> ratatui::text::Line<'a> {
    let normal = Style::default().fg(theme.status_bar_fg);
    let highlight = Style::default()
        .fg(theme.key_hint_fg)
        .add_modifier(ratatui::style::Modifier::BOLD);

    let is_dirty = app.model.left_dirty || app.model.right_dirty;

    let prefix = if app.model.change_count == 0 {
        " Files are identical  [".to_string()
    } else {
        format!(
            " {}/{}  [",
            app.model.current_block + 1,
            app.model.change_count,
        )
    };

    let mut spans = vec![Span::styled(prefix, normal)];

    if is_dirty {
        let q_active = app.input.pending_q
            && app
                .input
                .pending_q_at
                .map(|t| t.elapsed() <= crate::app::CHORD_TIMEOUT)
                .unwrap_or(false);

        let q_style = if q_active { highlight } else { normal };
        let both_dirty = app.model.left_dirty && app.model.right_dirty;

        if both_dirty {
            spans.push(Span::styled("w → save…", normal));
        } else {
            spans.push(Span::styled("w → save", normal));
            spans.push(Span::styled(" | ", normal));
            spans.push(Span::styled("wq → save & quit", normal));
        }
        spans.push(Span::styled(" | ", normal));
        spans.push(Span::styled("q", q_style));
        spans.push(Span::styled("! → force quit", normal));
    } else {
        spans.push(Span::styled("q → quit", normal));
    }

    spans.push(Span::styled("]", normal));

    ratatui::text::Line::from(spans)
}

/// Top-level UI: two file panes side by side + status bar.
pub fn draw(frame: &mut Frame, app: &mut App) {
    let [body, status] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());

    let (left_area, right_area, minimap_area) = if app.show_minimap {
        let [panes, minimap] =
            Layout::horizontal([Constraint::Min(0), Constraint::Length(MINIMAP_WIDTH)]).areas(body);

        let [left, right] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .spacing(1)
                .areas(panes);

        (left, right, Some(minimap))
    } else {
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .spacing(1)
                .areas(body);

        (left, right, None)
    };

    let max_lines = app
        .model
        .left_content
        .lines()
        .len()
        .max(app.model.right_content.lines().len());
    let digit_width = max_lines.to_string().len().max(2);
    let header_height = 3u16;

    // Update viewport dimensions early so initial scroll has correct bounds.
    let content_height = left_area.height.saturating_sub(header_height);
    let inner_height = content_height.saturating_sub(2);
    let gutter_cols = (digit_width as u16) + 2;
    let inner_code_width = left_area
        .width
        .saturating_sub(2)
        .saturating_sub(gutter_cols);
    app.viewport.height = inner_height;
    app.viewport.width = inner_code_width;
    app.viewport.clamp(
        app.model.display_rows.len(),
        app.model.max_content_width as u16,
    );

    // On first render, scroll to center the first change block.
    if app.needs_initial_scroll {
        app.needs_initial_scroll = false;
        crate::input::scroll_to_current_block(app);
    }

    // All mutation done — borrow theme for rendering.
    let theme = &app.theme;
    let max_content_width = app.model.max_content_width;
    let active_block_index = if app.model.change_block_indices.is_empty() {
        None
    } else {
        Some(app.model.change_block_indices[app.model.current_block])
    };

    render_file_pane(
        frame,
        left_area,
        &PaneContext {
            dir: &app.left_dir,
            filename: &app.left_filename,
            lines: app.model.left_content.lines(),
            display_rows: &app.model.display_rows,
            diff: &app.model.diff,
            side: Side::Left,
            scroll_y: app.viewport.scroll_y,
            scroll_x: app.viewport.scroll_x,
            digit_width,
            max_content_width,
            active_block_index,
            dirty: app.model.left_dirty,
            theme,
            tab_width: app.model.tab_width,
        },
    );
    render_file_pane(
        frame,
        right_area,
        &PaneContext {
            dir: &app.right_dir,
            filename: &app.right_filename,
            lines: app.model.right_content.lines(),
            display_rows: &app.model.display_rows,
            diff: &app.model.diff,
            side: Side::Right,
            scroll_y: app.viewport.scroll_y,
            scroll_x: app.viewport.scroll_x,
            digit_width,
            max_content_width,
            active_block_index,
            dirty: app.model.right_dirty,
            theme,
            tab_width: app.model.tab_width,
        },
    );

    // Pill indicator in the 1-column gap marking the current change block.
    if !app.model.change_block_indices.is_empty() {
        let block_index = app.model.change_block_indices[app.model.current_block];

        // Find the display row range for this block.
        let block_rows: Vec<usize> = app
            .model
            .display_rows
            .iter()
            .enumerate()
            .filter(|(_, r)| r.block_index == block_index)
            .map(|(i, _)| i)
            .collect();

        if !block_rows.is_empty() {
            let scroll_y = app.viewport.scroll_y as usize;
            let viewport_end = scroll_y + inner_height as usize;
            let gap_x = left_area.x + left_area.width;
            let pill_style = Style::default().fg(theme.gutter_dot);

            let buf_area = frame.area();
            for &row in &block_rows {
                if row >= scroll_y && row < viewport_end {
                    let screen_row = (row - scroll_y) as u16;
                    let gap_y = left_area.y + header_height + 1 + screen_row;
                    if gap_x < buf_area.width && gap_y < buf_area.y + buf_area.height {
                        frame.buffer_mut()[(gap_x, gap_y)]
                            .set_symbol("█")
                            .set_style(pill_style);
                    }
                }
            }
        }
    }

    // Minimap — aligned to the content viewport, not the full pane height.
    if let Some(minimap_area) = minimap_area {
        let content_top = header_height + 1; // header + top border
        let minimap_content = ratatui::layout::Rect {
            x: minimap_area.x,
            y: minimap_area.y + content_top,
            width: minimap_area.width,
            height: minimap_area
                .height
                .saturating_sub(content_top)
                .min(inner_height),
        };
        super::minimap::render(
            frame.buffer_mut(),
            minimap_content,
            &app.model.display_rows,
            app.viewport.scroll_y,
            app.viewport.height,
            active_block_index,
            theme,
        );
    }

    // Status bar.
    let hint_line = status_hint(app, theme);
    frame.render_widget(
        Paragraph::new(hint_line).alignment(Alignment::Center),
        status,
    );

    // Modal overlays render on top of everything else.
    match &app.overlay {
        Some(Overlay::WriteError { path, message }) => {
            overlay::render_write_error(frame, frame.area(), theme, path, message);
        }
        Some(Overlay::SavePicker) => {
            overlay::render_save_picker(frame, frame.area(), theme);
        }
        None => {}
    }
}
