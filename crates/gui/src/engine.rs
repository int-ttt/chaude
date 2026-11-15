//! Starts the shared core engine for the GUI and forwards its events to the UI thread.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use chaude_core::api::{Config, Engine, EngineHandle, UiEvent};
use chaude_core::handoff;
use chaude_core::tmp_layout;

// The exit handoff of a project (the TUI's <cwd>/notes/tmp/long/handoff file): the previous run loads from it.
pub fn handoff_path(cwd: &Path) -> PathBuf {
    handoff::handoff_file(&tmp_layout::long_dir(cwd))
}

// Starts the engine in the current directory. Boot blocks until ready; Err carries the boot failure text.
// A pump thread moves events to the returned channel and calls `wake` for each one (the window asks for a redraw).
pub fn start(
    wake: impl Fn() + Send + 'static,
) -> Result<(EngineHandle, Receiver<UiEvent>), String> {
    let cwd = std::env::current_dir().ok();
    let cfg = Config {
        handoff_path: cwd.as_deref().map(handoff_path),
        cwd,
        ..Config::default()
    };
    let (handle, core_rx) = Engine::try_start(cfg)?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        while let Ok(ev) = core_rx.recv() {
            if tx.send(ev).is_err() {
                break;
            }
            wake();
        }
    });
    Ok((handle, rx))
}
