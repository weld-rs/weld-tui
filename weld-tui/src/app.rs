use std::path::PathBuf;
use std::time::{Duration, Instant};

use weld_core::file::diff_model::DiffModel;
use weld_core::file::io::{Content, shorten_dir};

use crate::config::Config;
use crate::overlay::Overlay;
use crate::theme::Theme;
use crate::viewport::Viewport;

/// Application mode — determines how input is interpreted.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Variants used as features land (search, help overlay)
pub enum Mode {
    #[default]
    Normal,
    Command,
    Overlay,
}

/// Which file pane an action targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// Maximum time between keystrokes in a chord sequence (e.g., `q!`).
// TODO: make configurable via config.toml (#38)
pub const CHORD_TIMEOUT: Duration = Duration::from_millis(500);

/// Tracks multi-key input sequences (e.g., `gg`, `q!`).
#[derive(Default)]
pub struct InputState {
    /// Whether the previous keypress was `g` (waiting for `gg`).
    pub pending_g: bool,
    /// Whether the previous keypress was `q` on a dirty buffer (waiting for `!` to complete `q!`).
    pub pending_q: bool,
    /// When `pending_q` was set, for chord timeout.
    pub pending_q_at: Option<Instant>,
}

/// Top-level application state.
pub struct App {
    pub model: DiffModel,
    pub theme: Theme,
    pub running: bool,
    #[allow(dead_code)] // Used as features land (search, help overlay)
    pub mode: Mode,
    pub left_dir: String,
    pub left_filename: String,
    pub right_dir: String,
    pub right_filename: String,
    pub needs_initial_scroll: bool,
    pub viewport: Viewport,
    pub input: InputState,
    pub show_minimap: bool,
    /// Active overlay, if any. When `Some`, input routes to overlay handlers.
    pub overlay: Option<Overlay>,
    /// Files saved during this session, printed to stdout after exit.
    pub saved_files: Vec<String>,
}

impl App {
    pub fn new(left: PathBuf, right: PathBuf, config: Config) -> Result<Self, std::io::Error> {
        let left_content = Content::load(&left)?;
        let right_content = Content::load(&right)?;

        let left_abs = left.canonicalize().unwrap_or(left);
        let right_abs = right.canonicalize().unwrap_or(right);

        let mut app = Self::from_contents(left_content, right_content, config);
        app.left_dir = shorten_dir(
            &left_abs
                .parent()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
        );
        app.left_filename = left_abs
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        app.right_dir = shorten_dir(
            &right_abs
                .parent()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
        );
        app.right_filename = right_abs
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        app.needs_initial_scroll = true;

        Ok(app)
    }

    /// Construct an App from pre-loaded file contents (no filesystem access).
    pub fn from_contents(left_content: Content, right_content: Content, config: Config) -> Self {
        App {
            model: DiffModel::new(
                left_content,
                right_content,
                config.undo_capacity,
                config.tab_width,
            ),
            theme: Theme::default(),
            running: true,
            mode: Mode::default(),
            left_dir: String::new(),
            left_filename: String::new(),
            right_dir: String::new(),
            right_filename: String::new(),
            needs_initial_scroll: false,
            viewport: Viewport::default(),
            input: InputState::default(),
            show_minimap: config.show_minimap,
            overlay: None,
            saved_files: Vec::new(),
        }
    }
}
