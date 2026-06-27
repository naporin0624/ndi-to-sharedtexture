//! Minimal launcher GUI for ndi-share.

use eframe::egui;

struct GuiApp;

impl eframe::App for GuiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading(format!("NDI \u{2192} {}", ndi_share::output::output_kind()));
            ui.label("(launcher UI lands in the next tasks)");
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([420.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ndi-share",
        options,
        Box::new(|_cc| Ok(Box::new(GuiApp))),
    )
}
