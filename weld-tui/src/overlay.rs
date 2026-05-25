use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::theme::Theme;

/// Modal overlays that intercept input until dismissed.
#[derive(Debug, PartialEq)]
pub enum Overlay {
    /// Shown after a save failure. Displays the file path and OS error.
    /// Renders as a centered modal. Dismissed with `Esc`.
    WriteError { path: String, message: String },

    /// Shown when `w` is pressed with both sides dirty.
    /// Lets the user pick which side(s) to save: (l)eft, (r)ight, (a)ll, or Esc.
    SavePicker,
}

/// Render the `WriteError` modal centered in `area`.
pub fn render_write_error(frame: &mut Frame, area: Rect, theme: &Theme, path: &str, message: &str) {
    let modal = centered_rect(60, 9, area);

    let bg = Style::default().bg(theme.overlay_bg);
    let fg = Style::default().fg(theme.overlay_fg).bg(theme.overlay_bg);
    let title_style = Style::default()
        .fg(theme.overlay_fg)
        .bg(theme.overlay_bg)
        .add_modifier(Modifier::BOLD);

    frame.render_widget(Clear, modal);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(" Save failed ", title_style))
        .style(bg);

    let lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(format!("  {path}"), fg)]),
        Line::from(""),
        Line::from(vec![Span::styled(format!("  {message}"), fg)]),
        Line::from(""),
        Line::from(vec![Span::styled("  Esc to dismiss", fg)]),
    ];

    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        modal,
    );
}

/// Render the `SavePicker` modal centered in `area`.
pub fn render_save_picker(frame: &mut Frame, area: Rect, theme: &Theme) {
    let modal = centered_rect(40, 7, area);

    let bg = Style::default().bg(theme.overlay_bg);
    let fg = Style::default().fg(theme.overlay_fg).bg(theme.overlay_bg);
    let title_style = Style::default()
        .fg(theme.overlay_fg)
        .bg(theme.overlay_bg)
        .add_modifier(Modifier::BOLD);

    frame.render_widget(Clear, modal);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(" Save which side? ", title_style))
        .style(bg);

    let lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled("  l → left   r → right   a → all", fg)]),
        Line::from(""),
        Line::from(vec![Span::styled("  Esc to cancel", fg)]),
    ];

    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        modal,
    );
}

/// Compute a centered rectangle: `percent_x` percent wide, `height` rows tall.
fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let [_, middle, _] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(height),
        Constraint::Min(0),
    ])
    .areas(area);
    let left = (100 - percent_x) / 2;
    let right = 100 - percent_x - left;
    let [_, modal, _] = Layout::horizontal([
        Constraint::Percentage(left),
        Constraint::Percentage(percent_x),
        Constraint::Percentage(right),
    ])
    .areas(middle);
    modal
}
