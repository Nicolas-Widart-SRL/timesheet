//! README screenshots, rendered headlessly from `tests/fixtures/screenshots.json`.
//! Run `cargo test screenshots -- --ignored` to refresh `docs/screenshots/`.

use std::path::Path;

use chrono::NaiveDate;
use eframe::egui::Key;
use eframe::egui::accesskit::Role;
use egui_kittest::{Harness, kittest::Queryable as _};

use super::{App, View};
use crate::model::{Appearance, Kind};

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

#[test]
fn task_type_menu_changes_the_line() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(dir.path());
    h.run();
    h.get_by_role_and_label(Role::ComboBox, "Task type, Monday line 1: Backend")
        .click();
    h.run();
    h.get_by_role_and_label(Role::RadioButton, "Support")
        .click();
    h.run();

    let monday = NaiveDate::from_ymd_opt(2026, 10, 12).unwrap();
    let kinds: Vec<Kind> = h
        .state()
        .store
        .data
        .entries
        .iter()
        .filter(|e| e.date == monday)
        .map(|e| e.kind)
        .collect();
    assert_eq!(kinds, [Kind::Support]);
    assert!(
        h.query_by_role_and_label(Role::RadioButton, "Support")
            .is_none()
    );
}

#[test]
fn arrow_keys_step_hours_between_0_and_8() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(dir.path());
    h.run();
    h.get_by_role_and_label(Role::TextInput, "Hours, Monday line 1")
        .focus();
    h.run();

    let monday = NaiveDate::from_ymd_opt(2026, 10, 12).unwrap();
    let hours = |h: &Harness<'_, App>| {
        let entry = h
            .state()
            .store
            .data
            .entries
            .iter()
            .find(|e| e.date == monday);
        entry.unwrap().hours.clone()
    };
    assert_eq!(hours(&h), "8");
    h.key_press(Key::ArrowUp);
    h.run();
    assert_eq!(hours(&h), "8");
    h.key_press(Key::ArrowDown);
    h.run();
    assert_eq!(hours(&h), "7");
    for _ in 0..8 {
        h.key_press(Key::ArrowDown);
        h.run();
    }
    assert_eq!(hours(&h), "0");
    h.key_press(Key::ArrowUp);
    h.run();
    assert_eq!(hours(&h), "1");
}
