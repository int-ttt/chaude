//! chaude GUI entry point. Opens an empty window; the UI is added in later steps.

fn main() -> eframe::Result {
    eframe::run_ui_native("chaude", eframe::NativeOptions::default(), |_ui, _frame| {})
}
