//! Starts the shared core engine for the GUI and forwards its events to the UI thread.

use std::sync::mpsc::{self, Receiver};

use chaude_core::api::{Config, Engine, EngineHandle, UiEvent};
use eframe::egui;

// Starts the engine in the current directory. Boot blocks until ready; Err carries the boot failure text.
// A pump thread moves events to the returned channel and wakes egui for each one.
pub fn start(ctx: egui::Context) -> Result<(EngineHandle, Receiver<UiEvent>), String> {
    let cfg = Config {
        cwd: std::env::current_dir().ok(),
        ..Config::default()
    };
    let (handle, core_rx) = Engine::try_start(cfg)?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        while let Ok(ev) = core_rx.recv() {
            if tx.send(ev).is_err() {
                break;
            }
            ctx.request_repaint();
        }
    });
    Ok((handle, rx))
}
