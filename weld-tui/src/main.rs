mod app;
mod config;
mod event;
mod file_diff;
mod input;
mod overlay;
mod theme;
mod viewport;

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;
use crossterm::event::{Event, KeyEventKind};

use crate::app::App;
use crate::config::Config;

#[derive(Parser)]
#[command(name = "weld", version, about = "TUI diff and merge tool")]
struct Cli {
    /// Left file to compare
    left: PathBuf,
    /// Right file to compare
    right: PathBuf,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            report_error(&*e);
            ExitCode::FAILURE
        }
    }
}

/// Print an error and its `source()` chain using `Display` rather than `Debug`.
/// The default `Termination` impl for `Result<_, E>` uses `Debug`, which
/// bypasses our hand-written `Display` impls and produces unreadable output.
fn report_error(err: &(dyn std::error::Error + 'static)) {
    eprintln!("error: {err}");
    let mut source = err.source();
    while let Some(e) = source {
        eprintln!("  caused by: {e}");
        source = e.source();
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let config = Config::load()?;

    let mut app = App::new(cli.left, cli.right, config)?;

    // Restore the terminal on panic so it doesn't stay in raw mode.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        default_hook(info);
    }));

    let mut terminal = ratatui::init();

    let result = main_loop(&mut terminal, &mut app);

    ratatui::restore();

    app.saved_files.sort();
    app.saved_files.dedup();
    let left_path = app.model.left_content.path().display().to_string();
    for path in &app.saved_files {
        let (basename, side) = if path == &left_path {
            (&app.left_filename, "left")
        } else {
            (&app.right_filename, "right")
        };
        println!("{basename} ({side}): saved");
    }

    result
}

fn main_loop(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    while app.running {
        terminal.draw(|frame| file_diff::view::draw(frame, &mut *app))?;

        if let Some(Event::Key(key)) = event::poll_event(Duration::from_millis(50))?
            && key.kind == KeyEventKind::Press
        {
            input::handle_key(app, key);
        }
    }
    Ok(())
}
