//! chaude GUI entry point. Starts the core engine and opens the app window.

fn main() -> eframe::Result {
    eframe::run_native(
        "chaude",
        eframe::NativeOptions::default(),
        Box::new(|cc| {
            Ok(Box::new(chaude_gui::app::ChaudeApp::new(
                cc.egui_ctx.clone(),
            )))
        }),
    )
}
