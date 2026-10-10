//! README screenshots, rendered headlessly from `tests/fixtures/screenshots.json`.
//! Run `cargo test screenshots -- --ignored` to refresh `docs/screenshots/`.

use std::path::Path;

use chrono::NaiveDate;
use egui_kittest::Harness;

use super::{App, View};
use crate::model::Appearance;

fn harness(dir: &Path) -> Harness<'static, App> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let data_path = dir.join("timesheet.json");
    std::fs::copy(root.join("tests/fixtures/screenshots.json"), &data_path).unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap();
    Harness::builder()
        .with_size([1400.0, 940.0])
        .with_pixels_per_point(2.0)
        .build_eframe(move |cc| App::with(cc, data_path.clone(), today))
}

fn save(harness: &mut Harness<'_, App>, name: &str) {
    harness.run();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("docs/screenshots/{name}.png"));
    harness.render().unwrap().save(path).unwrap();
}

#[test]
#[ignore = "writes README screenshots to docs/screenshots"]
fn screenshots() {
    let dir = tempfile::tempdir().unwrap();

    let mut h = harness(dir.path());
    save(&mut h, "timesheet");

    h.state_mut().store.data.settings.appearance = Appearance::Dark;
    h.state_mut().week = 1;
    save(&mut h, "timesheet-dark");

    let mut h = harness(dir.path());
    h.state_mut().view = View::PastMonths;
    save(&mut h, "past-months");

    let mut h = harness(dir.path());
    h.state_mut().settings_draft = Some(h.state().store.data.settings.clone());
    save(&mut h, "settings");
}
