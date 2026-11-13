//! The eframe app: drains engine events into GuiState once per frame and draws them.

use std::sync::mpsc::Receiver;
use std::time::Instant;

use chaude_core::api::{EngineHandle, UiEvent};
use eframe::egui;

use crate::engine;
use crate::state::{Entry, GuiState};

pub struct ChaudeApp {
    // Kept alive for the app's lifetime; dropping it shuts the engine down.
    _engine: Option<EngineHandle>,
    rx: Option<Receiver<UiEvent>>,
    state: GuiState,
}

impl ChaudeApp {
    pub fn new(ctx: egui::Context) -> Self {
        let mut state = GuiState::default();
        match engine::start(ctx) {
            Ok((handle, rx)) => Self {
                _engine: Some(handle),
                rx: Some(rx),
                state,
            },
            Err(e) => {
                state.apply(UiEvent::Error(e), Instant::now());
                Self {
                    _engine: None,
                    rx: None,
                    state,
                }
            }
        }
    }
}

impl eframe::App for ChaudeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(rx) = &self.rx {
            while let Ok(ev) = rx.try_recv() {
                self.state.apply(ev, Instant::now());
            }
        }
        egui::CentralPanel::default().show(ui, |ui| {
            for entry in &self.state.transcript {
                if let Some(line) = plain_line(entry) {
                    ui.label(line);
                }
            }
        });
    }
}

// Temporary plain-text view until the output pane (G1 commit 2) renders markdown.
fn plain_line(entry: &Entry) -> Option<String> {
    match entry {
        Entry::User(t) => Some(format!("> {t}")),
        Entry::Text { text, .. } => Some(text.clone()),
        Entry::Notice(t) => Some(t.clone()),
        Entry::Error(t) => Some(format!("error: {t}")),
        _ => None,
    }
}
