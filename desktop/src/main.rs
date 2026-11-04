mod engine;
mod model;
mod ui;

use ui::CheatClip;

fn main() -> eframe::Result<()> {
    let initial = std::env::args().nth(1);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([960.0, 640.0])
            .with_title("Cheat Clip"),
        ..Default::default()
    };
    eframe::run_native(
        "Cheat Clip",
        options,
        Box::new(move |cc| Ok(Box::new(CheatClip::new(cc, initial.clone())))),
    )
}
