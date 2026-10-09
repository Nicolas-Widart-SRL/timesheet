#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod fonts;
mod holidays;
mod model;
mod pdf;
mod ui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Timesheet")
            .with_inner_size([1400.0, 940.0])
            .with_min_inner_size([1000.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Timesheet",
        options,
        Box::new(|cc| Ok(Box::new(ui::App::new(cc)))),
    )
}
