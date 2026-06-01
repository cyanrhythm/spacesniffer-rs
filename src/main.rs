mod app;
mod scanner;
mod treemap;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([640.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "SpaceSnifferX",
        options,
        Box::new(|_cc| Ok(Box::new(app::SpaceSnifferApp::new()))),
    )
}
