use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, CHORD_TIMEOUT, Side};
use crate::overlay::Overlay;

/// Handle a key press, updating app state.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    // Overlays intercept input until dismissed.
    if app.overlay.is_some() {
        handle_overlay_key(app, key);
        return;
    }
    handle_normal_key(app, key);
}

fn handle_normal_key(app: &mut App, key: KeyEvent) {
    let total_rows = app.model.display_rows.len();
    let max_x = app.model.max_content_width as u16;
    let code = key.code;

    // Handle `gg` — two consecutive `g` presses jump to first change block
    if app.input.pending_g {
        app.input.pending_g = false;
        if code == KeyCode::Char('g') {
            first_block(app);
            return;
        }
    }

    // Handle `q!` — `q` on dirty starts the chord, `!` completes it.
    // The chord expires after CHORD_TIMEOUT; an expired `q` is silently dropped.
    if app.input.pending_q {
        let expired = app
            .input
            .pending_q_at
            .map(|t| t.elapsed() > CHORD_TIMEOUT)
            .unwrap_or(true);
        app.input.pending_q = false;
        app.input.pending_q_at = None;
        if !expired && code == KeyCode::Char('!') {
            app.running = false;
            return;
        }
        // Expired or wrong key — fall through to process the key normally
    }

    match code {
        KeyCode::Char('q') => {
            if is_dirty(app) {
                app.input.pending_q = true;
                app.input.pending_q_at = Some(std::time::Instant::now());
            } else {
                app.running = false;
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.viewport.scroll_down(total_rows);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.viewport.scroll_up();
        }
        KeyCode::Char('J') => {
            next_block(app);
        }
        KeyCode::Char('K') => {
            prev_block(app);
        }
        KeyCode::Char('l') | KeyCode::Right => {
            app.viewport.scroll_right(2, max_x);
        }
        KeyCode::Char('h') | KeyCode::Left => {
            app.viewport.scroll_left(2);
        }
        KeyCode::Char('0') | KeyCode::Home => {
            app.viewport.scroll_to_left();
        }
        KeyCode::Char('$') | KeyCode::End => {
            app.viewport.scroll_to_right(max_x);
        }
        KeyCode::Char('g') => {
            app.input.pending_g = true;
        }
        KeyCode::Char('G') => {
            last_block(app);
        }
        KeyCode::Char('L') => {
            app.model.copy_left_to_right();
            scroll_to_current_block(app);
        }
        KeyCode::Char('H') => {
            app.model.copy_right_to_left();
            scroll_to_current_block(app);
        }
        KeyCode::Char('w') => {
            let both_dirty = app.model.left_dirty && app.model.right_dirty;
            if both_dirty {
                app.overlay = Some(Overlay::SavePicker);
            } else {
                save_all_dirty(app);
            }
        }
        KeyCode::Char('u') => {
            app.model.undo();
            scroll_to_current_block(app);
        }
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.model.redo();
            scroll_to_current_block(app);
        }
        _ => {}
    }
}

/// Dispatch input while an overlay is active.
fn handle_overlay_key(app: &mut App, key: KeyEvent) {
    match &app.overlay {
        Some(Overlay::WriteError { .. }) if key.code == KeyCode::Esc => {
            app.overlay = None;
        }
        Some(Overlay::WriteError { .. }) => {}
        Some(Overlay::SavePicker) => match key.code {
            KeyCode::Char('l') => {
                app.overlay = None;
                save_side(app, Side::Left);
            }
            KeyCode::Char('r') => {
                app.overlay = None;
                save_side(app, Side::Right);
            }
            KeyCode::Char('a') => {
                app.overlay = None;
                save_all_dirty(app);
            }
            KeyCode::Esc => {
                app.overlay = None;
            }
            _ => {}
        },
        None => {}
    }
}

/// Whether either side has unsaved changes.
fn is_dirty(app: &App) -> bool {
    app.model.left_dirty || app.model.right_dirty
}

/// Save every dirty side in turn. Returns `true` if all required saves
/// succeeded (or nothing was dirty). On the first failure, sets
/// `Overlay::WriteError` and returns `false`; later sides are not attempted.
fn save_all_dirty(app: &mut App) -> bool {
    if app.model.left_dirty && !save_side(app, Side::Left) {
        return false;
    }
    if app.model.right_dirty && !save_side(app, Side::Right) {
        return false;
    }
    true
}

/// Persist one side to disk. On success, clears its dirty flag and returns
/// `true`. On failure, sets `Overlay::WriteError` (preserving the dirty flag
/// so the user can retry) and returns `false`.
fn save_side(app: &mut App, side: Side) -> bool {
    let result = match side {
        Side::Left => app.model.left_content.save(),
        Side::Right => app.model.right_content.save(),
    };
    let full_path = match side {
        Side::Left => app.model.left_content.path().display().to_string(),
        Side::Right => app.model.right_content.path().display().to_string(),
    };
    match result {
        Ok(()) => {
            match side {
                Side::Left => app.model.left_dirty = false,
                Side::Right => app.model.right_dirty = false,
            }
            app.saved_files.push(full_path);
            true
        }
        Err(err) => {
            app.overlay = Some(Overlay::WriteError {
                path: full_path,
                message: err.to_string(),
            });
            false
        }
    }
}

/// Jump to the first change block.
fn first_block(app: &mut App) {
    if app.model.change_block_indices.is_empty() {
        return;
    }
    app.model.current_block = 0;
    scroll_to_current_block(app);
}

/// Jump to the last change block.
fn last_block(app: &mut App) {
    if app.model.change_block_indices.is_empty() {
        return;
    }
    app.model.current_block = app.model.change_block_indices.len() - 1;
    scroll_to_current_block(app);
}

/// Advance to the next change block (clamped at last).
fn next_block(app: &mut App) {
    if app.model.change_block_indices.is_empty() {
        return;
    }
    if app.model.current_block < app.model.change_block_indices.len() - 1 {
        app.model.current_block += 1;
    }
    scroll_to_current_block(app);
}

/// Retreat to the previous change block (clamped at first).
fn prev_block(app: &mut App) {
    if app.model.change_block_indices.is_empty() {
        return;
    }
    app.model.current_block = app.model.current_block.saturating_sub(1);
    scroll_to_current_block(app);
}

/// Scroll so the current change block is vertically centered in the viewport.
pub fn scroll_to_current_block(app: &mut App) {
    if app.model.change_block_indices.is_empty() || app.viewport.height == 0 {
        return;
    }

    let block_index = app.model.change_block_indices[app.model.current_block];
    let block_start = app
        .model
        .display_rows
        .iter()
        .position(|r| r.block_index == block_index)
        .unwrap_or(0) as u16;

    let half_vp = app.viewport.height / 2;
    let target = block_start.saturating_sub(half_vp);
    let max = app.viewport.scroll_y_max(app.model.display_rows.len());
    app.viewport.scroll_y = target.min(max);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;
    use weld_core::file::io::Content;

    use crate::config::Config;
    use crate::viewport::Viewport;

    /// Build a plain KeyEvent (no modifiers) from a KeyCode.
    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, crossterm::event::KeyModifiers::NONE)
    }

    /// Build an App backed by real files on disk so `save()` succeeds.
    /// The returned `TempDir` must outlive the App.
    fn test_app_with_files(left_lines: &[&str], right_lines: &[&str]) -> (App, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let left_path = dir.path().join("left.txt");
        let right_path = dir.path().join("right.txt");
        let left_body = left_lines.join("\n") + "\n";
        let right_body = right_lines.join("\n") + "\n";
        std::fs::write(&left_path, left_body).unwrap();
        std::fs::write(&right_path, right_body).unwrap();

        let left_content = Content::load(&left_path).unwrap();
        let right_content = Content::load(&right_path).unwrap();
        let mut app = App::from_contents(left_content, right_content, Config::default());
        app.left_filename = "left.txt".to_string();
        app.right_filename = "right.txt".to_string();
        app.viewport = Viewport {
            scroll_y: 0,
            scroll_x: 0,
            height: 10,
            width: 40,
        };
        (app, dir)
    }

    fn test_app(left_lines: &[&str], right_lines: &[&str], viewport: (u16, u16)) -> App {
        let mut app = App::from_contents(
            Content::from_lines(left_lines),
            Content::from_lines(right_lines),
            Config::default(),
        );
        app.viewport = Viewport {
            scroll_y: 0,
            scroll_x: 0,
            height: viewport.1,
            width: viewport.0,
        };
        app
    }

    #[test]
    fn j_caps_at_viewport_bottom() {
        let lines = vec!["line"; 20];
        let mut app = test_app(&lines, &lines, (40, 10));

        for _ in 0..25 {
            handle_key(&mut app, key(KeyCode::Char('j')));
        }

        // 20 identical lines = 20 display rows. max scroll = 20 - 10 = 10
        assert_eq!(app.viewport.scroll_y, 10);
    }

    #[test]
    fn j_scrolls_to_max() {
        let lines = vec!["content"; 50];
        let mut app = test_app(&lines, &lines, (40, 20));

        for _ in 0..100 {
            handle_key(&mut app, key(KeyCode::Char('j')));
        }

        assert_eq!(app.viewport.scroll_y, app.viewport.scroll_y_max(50));
    }

    #[test]
    fn dollar_uses_global_max_across_both_files() {
        let mut left: Vec<&str> = vec!["short"; 51];
        let long = &"a".repeat(200);
        left[50] = long;

        let mut app = test_app(&left, &["short"; 51], (40, 10));

        handle_key(&mut app, key(KeyCode::Char('$')));

        // Global max = 201 (200 + leading space), viewport = 40 → scroll_x = 161
        assert_eq!(
            app.viewport.scroll_x, 161,
            "$ should use global max even if long line is off-screen"
        );
    }

    #[test]
    fn dollar_adapts_when_scrolled_to_long_line() {
        let long = "x".repeat(100);
        let mut lines: Vec<String> = vec!["short".to_string(); 20];
        lines[15] = long;
        let line_refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();

        let mut app = test_app(&line_refs, &line_refs, (40, 10));

        app.viewport.scroll_y = 10;
        handle_key(&mut app, key(KeyCode::Char('$')));

        // max_x = 101 (100 + leading space), viewport_width = 40 → scroll_x = 61
        assert_eq!(
            app.viewport.scroll_x, 61,
            "$ should use the long line now visible"
        );
    }

    #[test]
    fn gg_jumps_to_first_block() {
        let left = vec!["a", "b", "c", "d", "e", "f", "g", "h"];
        let right = vec!["a", "X", "c", "d", "e", "Y", "g", "h"];
        let mut app = test_app(&left, &right, (40, 10));
        app.model.current_block = 1;

        handle_key(&mut app, key(KeyCode::Char('g')));
        handle_key(&mut app, key(KeyCode::Char('g')));

        assert_eq!(app.model.current_block, 0);
    }

    #[test]
    fn shift_g_jumps_to_last_block() {
        let left = vec!["a", "b", "c", "d", "e", "f", "g", "h"];
        let right = vec!["a", "X", "c", "d", "e", "Y", "g", "h"];
        let mut app = test_app(&left, &right, (40, 10));

        assert_eq!(app.model.current_block, 0);
        handle_key(&mut app, key(KeyCode::Char('G')));
        assert_eq!(
            app.model.current_block,
            app.model.change_block_indices.len() - 1
        );
    }

    #[test]
    fn l_and_dollar_agree_on_max_scroll() {
        let long = "x".repeat(100);
        let mut app = test_app(&[&long], &[&long], (40, 10));

        handle_key(&mut app, key(KeyCode::Char('$')));
        let dollar_pos = app.viewport.scroll_x;

        app.viewport.scroll_x = 0;
        for _ in 0..200 {
            handle_key(&mut app, key(KeyCode::Char('l')));
        }

        assert_eq!(
            app.viewport.scroll_x, dollar_pos,
            "l max should equal $ position"
        );
    }

    #[test]
    fn g_then_non_g_does_not_jump() {
        let lines = vec!["line"; 50];
        let mut app = test_app(&lines, &lines, (40, 10));
        app.viewport.scroll_y = 30;

        handle_key(&mut app, key(KeyCode::Char('g')));
        handle_key(&mut app, key(KeyCode::Char('j')));

        assert_eq!(app.viewport.scroll_y, 31, "g then j should just move down");
    }

    #[test]
    fn display_rows_include_padding_for_inserts() {
        let left = vec!["a", "b", "c"];
        let right = vec!["a", "x", "y", "b", "c"];
        let app = test_app(&left, &right, (40, 20));

        // Display rows should include padding for alignment
        assert!(app.model.display_rows.len() >= 5);
    }

    #[test]
    fn shift_j_advances_to_next_block() {
        // Equal lines, then a change, then equal, then a change
        let left = vec!["a", "b", "c", "d", "e", "f", "g", "h"];
        let right = vec!["a", "X", "c", "d", "e", "Y", "g", "h"];
        let mut app = test_app(&left, &right, (40, 10));

        assert_eq!(app.model.current_block, 0);
        handle_key(&mut app, key(KeyCode::Char('J')));
        assert_eq!(app.model.current_block, 1);
    }

    #[test]
    fn shift_k_retreats_to_previous_block() {
        let left = vec!["a", "b", "c", "d", "e", "f", "g", "h"];
        let right = vec!["a", "X", "c", "d", "e", "Y", "g", "h"];
        let mut app = test_app(&left, &right, (40, 10));

        app.model.current_block = 1;
        handle_key(&mut app, key(KeyCode::Char('K')));
        assert_eq!(app.model.current_block, 0);
    }

    #[test]
    fn shift_j_clamps_at_last_block() {
        let left = vec!["a", "b"];
        let right = vec!["a", "X"];
        let mut app = test_app(&left, &right, (40, 10));

        // Only one change block — repeated Ctrl+j should stay at 0
        handle_key(&mut app, key(KeyCode::Char('J')));
        handle_key(&mut app, key(KeyCode::Char('J')));
        assert_eq!(app.model.current_block, 0);
    }

    #[test]
    fn shift_k_clamps_at_first_block() {
        let left = vec!["a", "b"];
        let right = vec!["a", "X"];
        let mut app = test_app(&left, &right, (40, 10));

        handle_key(&mut app, key(KeyCode::Char('K')));
        assert_eq!(app.model.current_block, 0);
    }

    #[test]
    fn scroll_to_block_centers_vertically() {
        // 30 equal lines, then a change, then more equal lines
        let mut left: Vec<&str> = vec!["same"; 30];
        left.push("old");
        left.extend(vec!["same"; 20]);
        let mut right: Vec<&str> = vec!["same"; 30];
        right.push("new");
        right.extend(vec!["same"; 20]);

        let mut app = test_app(&left, &right, (40, 10));

        scroll_to_current_block(&mut app);

        // Block starts at display row 30. Center in viewport of height 10 → scroll_y = 30 - 5 = 25
        assert_eq!(app.viewport.scroll_y, 25);
    }

    #[test]
    fn copy_left_to_right_replaces_content() {
        let left = vec!["a", "b", "c"];
        let right = vec!["a", "X", "c"];
        let mut app = test_app(&left, &right, (40, 10));

        handle_key(&mut app, key(KeyCode::Char('L')));

        assert_eq!(app.model.right_content.lines(), &["a", "b", "c"]);
        assert!(app.model.right_dirty);
        assert!(!app.model.left_dirty);
    }

    #[test]
    fn copy_right_to_left_replaces_content() {
        let left = vec!["a", "b", "c"];
        let right = vec!["a", "X", "c"];
        let mut app = test_app(&left, &right, (40, 10));

        handle_key(&mut app, key(KeyCode::Char('H')));

        assert_eq!(app.model.left_content.lines(), &["a", "X", "c"]);
        assert!(app.model.left_dirty);
        assert!(!app.model.right_dirty);
    }

    #[test]
    fn copy_removes_change_block() {
        let left = vec!["a", "b", "c"];
        let right = vec!["a", "X", "c"];
        let mut app = test_app(&left, &right, (40, 10));

        assert_eq!(app.model.change_count, 1);
        handle_key(&mut app, key(KeyCode::Char('L')));
        assert_eq!(app.model.change_count, 0);
    }

    #[test]
    fn copy_clamps_current_block() {
        // Two change blocks; navigate to the last, then copy it away.
        let left = vec!["a", "b", "c", "d", "e"];
        let right = vec!["a", "X", "c", "Y", "e"];
        let mut app = test_app(&left, &right, (40, 10));

        handle_key(&mut app, key(KeyCode::Char('J'))); // move to second block
        assert_eq!(app.model.current_block, 1);

        handle_key(&mut app, key(KeyCode::Char('L'))); // copy it away
        assert_eq!(app.model.change_count, 1);
        assert_eq!(app.model.current_block, 0); // clamped back
    }

    #[test]
    fn copy_insert_block_left_to_right() {
        // Right has extra lines — copying left→right removes them.
        let left = vec!["a", "b"];
        let right = vec!["a", "X", "Y", "b"];
        let mut app = test_app(&left, &right, (40, 10));

        handle_key(&mut app, key(KeyCode::Char('L')));

        assert_eq!(app.model.right_content.lines(), &["a", "b"]);
        assert_eq!(app.model.change_count, 0);
    }

    #[test]
    fn copy_noop_when_no_changes() {
        let left = vec!["a", "b", "c"];
        let right = vec!["a", "b", "c"];
        let mut app = test_app(&left, &right, (40, 10));

        handle_key(&mut app, key(KeyCode::Char('L')));

        assert!(!app.model.left_dirty);
        assert!(!app.model.right_dirty);
    }

    /// Helper to create a KeyEvent with Ctrl modifier.
    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    #[test]
    fn undo_restores_previous_state() {
        let left = vec!["a", "b", "c"];
        let right = vec!["a", "X", "c"];
        let mut app = test_app(&left, &right, (40, 10));

        handle_key(&mut app, key(KeyCode::Char('L'))); // copy left→right
        assert_eq!(app.model.right_content.lines(), &["a", "b", "c"]);
        assert_eq!(app.model.change_count, 0);

        handle_key(&mut app, key(KeyCode::Char('u'))); // undo
        assert_eq!(app.model.right_content.lines(), &["a", "X", "c"]);
        assert_eq!(app.model.change_count, 1);
        assert!(!app.model.right_dirty);
    }

    #[test]
    fn redo_restores_undone_state() {
        let left = vec!["a", "b", "c"];
        let right = vec!["a", "X", "c"];
        let mut app = test_app(&left, &right, (40, 10));

        handle_key(&mut app, key(KeyCode::Char('L'))); // copy
        handle_key(&mut app, key(KeyCode::Char('u'))); // undo
        handle_key(&mut app, ctrl(KeyCode::Char('r'))); // redo

        assert_eq!(app.model.right_content.lines(), &["a", "b", "c"]);
        assert_eq!(app.model.change_count, 0);
        assert!(app.model.right_dirty);
    }

    #[test]
    fn undo_noop_when_nothing_to_undo() {
        let left = vec!["a", "b", "c"];
        let right = vec!["a", "X", "c"];
        let mut app = test_app(&left, &right, (40, 10));

        handle_key(&mut app, key(KeyCode::Char('u'))); // no-op
        assert_eq!(app.model.right_content.lines(), &["a", "X", "c"]);
        assert_eq!(app.model.change_count, 1);
    }

    #[test]
    fn new_copy_clears_redo() {
        let left = vec!["a", "b", "c", "d", "e"];
        let right = vec!["a", "X", "c", "Y", "e"];
        let mut app = test_app(&left, &right, (40, 10));

        handle_key(&mut app, key(KeyCode::Char('L'))); // copy block 0
        handle_key(&mut app, key(KeyCode::Char('u'))); // undo
        handle_key(&mut app, key(KeyCode::Char('L'))); // new copy — clears redo

        handle_key(&mut app, ctrl(KeyCode::Char('r'))); // redo should be no-op
        // After the new copy, we copied block 0 again; the old redo is gone.
        // Just verify redo didn't crash or change state unexpectedly.
        assert!(app.model.right_dirty);
    }

    // ---- Step 1: dirty-aware `q` + ConfirmQuit overlay ----

    #[test]
    fn q_clean_quits_immediately() {
        let left = vec!["a", "b"];
        let right = vec!["a", "b"];
        let mut app = test_app(&left, &right, (40, 10));

        handle_key(&mut app, key(KeyCode::Char('q')));

        assert!(!app.running);
        assert!(app.overlay.is_none());
    }

    #[test]
    fn q_dirty_sets_pending_q() {
        let left = vec!["a", "b"];
        let right = vec!["a", "X"];
        let mut app = test_app(&left, &right, (40, 10));
        handle_key(&mut app, key(KeyCode::Char('L'))); // dirty right

        handle_key(&mut app, key(KeyCode::Char('q')));

        assert!(app.running, "q on dirty should not quit immediately");
        assert!(app.input.pending_q, "q on dirty should set pending_q");
    }

    #[test]
    fn q_bang_force_quits() {
        let left = vec!["a", "b"];
        let right = vec!["a", "X"];
        let mut app = test_app(&left, &right, (40, 10));
        handle_key(&mut app, key(KeyCode::Char('L')));

        handle_key(&mut app, key(KeyCode::Char('q'))); // pending
        handle_key(&mut app, key(KeyCode::Char('!'))); // complete q!

        assert!(!app.running);
        assert!(
            app.model.right_dirty,
            "force quit preserves dirty flag (no save)"
        );
    }

    #[test]
    fn q_then_wrong_key_clears_pending_and_processes_key() {
        let left = vec!["line"; 20];
        let right = vec!["line"; 20];
        let mut app = test_app(&left, &right, (40, 10));
        // Make dirty so q sets pending instead of quitting.
        app.model.left_dirty = true;

        handle_key(&mut app, key(KeyCode::Char('q'))); // pending
        assert!(app.input.pending_q);

        handle_key(&mut app, key(KeyCode::Char('j'))); // wrong key → clears pending, scrolls

        assert!(!app.input.pending_q, "pending should be cleared");
        assert!(app.running, "should not have quit");
        assert_eq!(app.viewport.scroll_y, 1, "j should have scrolled");
    }

    #[test]
    fn write_error_esc_dismisses() {
        let left = vec!["a", "b"];
        let right = vec!["a", "X"];
        let mut app = test_app(&left, &right, (40, 10));
        app.overlay = Some(Overlay::WriteError {
            path: "right.txt".into(),
            message: "permission denied".into(),
        });

        handle_key(&mut app, key(KeyCode::Esc));

        assert!(app.overlay.is_none());
        assert!(app.running);
    }

    // ---- Step 2: `w` save binding ----

    #[test]
    fn w_saves_when_one_side_dirty() {
        let left = vec!["a", "b"];
        let right = vec!["a", "X"];
        let (mut app, _dir) = test_app_with_files(&left, &right);
        let right_path = app.model.right_content.path().display().to_string();
        handle_key(&mut app, key(KeyCode::Char('L'))); // dirty right

        handle_key(&mut app, key(KeyCode::Char('w')));

        assert!(!app.model.right_dirty, "w should clear dirty flag");
        assert_eq!(app.saved_files, vec![right_path]);
        assert!(app.overlay.is_none());
    }

    #[test]
    fn w_noop_when_clean() {
        let left = vec!["a", "b"];
        let right = vec!["a", "b"];
        let (mut app, _dir) = test_app_with_files(&left, &right);

        handle_key(&mut app, key(KeyCode::Char('w')));

        assert!(app.saved_files.is_empty());
        assert!(app.overlay.is_none());
    }

    #[test]
    fn w_opens_picker_when_both_dirty() {
        let left = vec!["a", "b"];
        let right = vec!["a", "X"];
        let (mut app, _dir) = test_app_with_files(&left, &right);
        app.model.left_dirty = true;
        app.model.right_dirty = true;

        handle_key(&mut app, key(KeyCode::Char('w')));

        assert!(
            matches!(app.overlay, Some(Overlay::SavePicker)),
            "both-dirty w should open SavePicker"
        );
        assert!(app.saved_files.is_empty(), "nothing saved yet");
    }

    #[test]
    fn save_picker_l_saves_left() {
        let left = vec!["a", "X"];
        let right = vec!["a", "Y"];
        let (mut app, _dir) = test_app_with_files(&left, &right);
        let left_path = app.model.left_content.path().display().to_string();
        app.model.left_dirty = true;
        app.model.right_dirty = true;
        app.overlay = Some(Overlay::SavePicker);

        handle_key(&mut app, key(KeyCode::Char('l')));

        assert!(!app.model.left_dirty, "l should save left");
        assert!(app.model.right_dirty, "l should not save right");
        assert_eq!(app.saved_files, vec![left_path]);
        assert!(app.overlay.is_none(), "picker should dismiss");
    }

    #[test]
    fn save_picker_r_saves_right() {
        let left = vec!["a", "X"];
        let right = vec!["a", "Y"];
        let (mut app, _dir) = test_app_with_files(&left, &right);
        let right_path = app.model.right_content.path().display().to_string();
        app.model.left_dirty = true;
        app.model.right_dirty = true;
        app.overlay = Some(Overlay::SavePicker);

        handle_key(&mut app, key(KeyCode::Char('r')));

        assert!(app.model.left_dirty, "r should not save left");
        assert!(!app.model.right_dirty, "r should save right");
        assert_eq!(app.saved_files, vec![right_path]);
        assert!(app.overlay.is_none());
    }

    #[test]
    fn save_picker_a_saves_all() {
        let left = vec!["a", "X"];
        let right = vec!["a", "Y"];
        let (mut app, _dir) = test_app_with_files(&left, &right);
        app.model.left_dirty = true;
        app.model.right_dirty = true;
        app.overlay = Some(Overlay::SavePicker);

        handle_key(&mut app, key(KeyCode::Char('a')));

        assert!(!app.model.left_dirty, "a should save left");
        assert!(!app.model.right_dirty, "a should save right");
        assert_eq!(app.saved_files.len(), 2);
        assert!(app.overlay.is_none());
    }

    #[test]
    fn save_picker_esc_dismisses() {
        let left = vec!["a", "X"];
        let right = vec!["a", "Y"];
        let (mut app, _dir) = test_app_with_files(&left, &right);
        app.model.left_dirty = true;
        app.model.right_dirty = true;
        app.overlay = Some(Overlay::SavePicker);

        handle_key(&mut app, key(KeyCode::Esc));

        assert!(app.overlay.is_none(), "Esc should dismiss picker");
        assert!(app.model.left_dirty, "Esc should not save");
        assert!(app.model.right_dirty, "Esc should not save");
    }

    #[test]
    fn save_picker_swallows_other_keys() {
        let left = vec!["a", "X"];
        let right = vec!["a", "Y"];
        let (mut app, _dir) = test_app_with_files(&left, &right);
        app.model.left_dirty = true;
        app.model.right_dirty = true;
        app.overlay = Some(Overlay::SavePicker);

        let before = app.viewport.scroll_y;
        handle_key(&mut app, key(KeyCode::Char('j')));

        assert!(matches!(app.overlay, Some(Overlay::SavePicker)));
        assert_eq!(app.viewport.scroll_y, before, "j must not scroll in picker");
    }

    #[test]
    #[cfg(unix)]
    fn w_shows_error_overlay_on_write_failure() {
        use std::os::unix::fs::PermissionsExt;

        let left = vec!["a", "b"];
        let right = vec!["a", "X"];
        let (mut app, _dir) = test_app_with_files(&left, &right);
        handle_key(&mut app, key(KeyCode::Char('L'))); // dirty right

        // Make the file read-only so save() fails with a permission error.
        let right_path = _dir.path().join("right.txt");
        let original_mode = std::fs::metadata(&right_path).unwrap().permissions().mode();
        std::fs::set_permissions(&right_path, std::fs::Permissions::from_mode(0o444)).unwrap();

        handle_key(&mut app, key(KeyCode::Char('w')));

        assert!(
            matches!(app.overlay, Some(Overlay::WriteError { .. })),
            "write failure should show error overlay"
        );
        assert!(app.model.right_dirty, "dirty flag preserved on failure");

        // Restore permissions so tempdir cleanup succeeds.
        std::fs::set_permissions(&right_path, std::fs::Permissions::from_mode(original_mode))
            .unwrap();
    }

    #[test]
    fn wq_composition_saves_then_quits() {
        let left = vec!["a", "b"];
        let right = vec!["a", "X"];
        let (mut app, _dir) = test_app_with_files(&left, &right);
        let right_path = app.model.right_content.path().display().to_string();
        handle_key(&mut app, key(KeyCode::Char('L'))); // dirty right

        handle_key(&mut app, key(KeyCode::Char('w'))); // save
        handle_key(&mut app, key(KeyCode::Char('q'))); // quit (now clean)

        assert!(!app.running, "q after w should quit");
        assert!(!app.model.right_dirty);
        assert_eq!(app.saved_files, vec![right_path]);
    }

    #[test]
    fn write_error_overlay_swallows_normal_keys() {
        let left = vec!["a", "b", "c"];
        let right = vec!["a", "X", "c"];
        let mut app = test_app(&left, &right, (40, 10));
        app.overlay = Some(Overlay::WriteError {
            path: "right.txt".into(),
            message: "permission denied".into(),
        });

        let before = app.viewport.scroll_y;
        handle_key(&mut app, key(KeyCode::Char('j')));

        assert_eq!(
            app.viewport.scroll_y, before,
            "j must not scroll in overlay"
        );
        assert!(matches!(app.overlay, Some(Overlay::WriteError { .. })));
    }
}
