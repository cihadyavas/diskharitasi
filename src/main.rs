mod app;
mod i18n;
mod layout;
mod scan;
mod tree;

use std::path::PathBuf;

fn main() -> eframe::Result {
    let arg = std::env::args_os().nth(1).map(PathBuf::from);
    if arg.as_ref().is_some_and(|a| a.to_string_lossy().starts_with('-')) {
        println!("Kullanım: diskharitasi [KLASÖR]");
        return Ok(());
    }
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([480.0, 320.0])
            .with_title("Disk Haritası")
            .with_app_id("diskharitasi"),
        ..Default::default()
    };
    eframe::run_native(
        "diskharitasi",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, arg)))),
    )
}
