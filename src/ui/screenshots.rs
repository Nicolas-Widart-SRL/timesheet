//! README screenshots and keyboard tests, run headlessly on `tests/fixtures/screenshots.json`.
//! Run `cargo test screenshots -- --ignored` to refresh `docs/screenshots/`.

use std::path::Path;

use chrono::NaiveDate;
use eframe::egui::accesskit::Role;
use eframe::egui::{Event, Key, Modifiers};
use egui_kittest::{
    Harness,
    kittest::{NodeT as _, Queryable as _},
};

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

#[test]
fn tab_follows_the_line_then_moves_to_the_next_day() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(dir.path());
    h.run();
    h.get_by_role_and_label(Role::ComboBox, "Task type, Tuesday line 2: Support")
        .focus();
    h.run();

    for label in [
        "Description, Tuesday line 2",
        "Hours, Tuesday line 2",
        "Remove Tuesday line 2",
    ] {
        h.key_press(Key::Tab);
        h.run();
        assert_eq!(focused(&h), label);
    }
    h.key_press(Key::Tab);
    h.run();
    assert!(focused(&h).starts_with("Add line"), "{}", focused(&h));
}

fn focused(h: &Harness<'_, App>) -> String {
    let node = h.get_by(|n| n.is_focused());
    node.accesskit_node().label().unwrap_or_default()
}

fn press(h: &mut Harness<'_, App>, modifiers: Modifiers, key: Key) {
    h.key_press_modifiers(modifiers, key);
    h.run();
}

fn oct(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
}

#[test]
fn enter_a_day_without_the_mouse() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(dir.path());
    h.run();

    // Wednesday 14 is empty: Cmd+3 focuses "Add line", Enter adds a line and focuses it.
    press(&mut h, Modifiers::COMMAND, Key::Num3);
    assert!(focused(&h).starts_with("Add line"), "{}", focused(&h));
    press(&mut h, Modifiers::NONE, Key::Enter);
    assert_eq!(focused(&h), "Description, Wednesday line 1");
    h.event(Event::Text("Ticket #1400".into()));
    h.run();
    press(&mut h, Modifiers::NONE, Key::Enter);
    assert_eq!(focused(&h), "Hours, Wednesday line 1");

    // Under 8 h, Enter on the last line adds one.
    press(&mut h, Modifiers::NONE, Key::ArrowDown);
    press(&mut h, Modifiers::NONE, Key::ArrowDown);
    press(&mut h, Modifiers::NONE, Key::Enter);
    assert_eq!(focused(&h), "Description, Wednesday line 2");
    let lines: Vec<(String, String)> = h
        .state()
        .data()
        .lines(oct(14))
        .map(|e| (e.desc.clone(), e.hours.clone()))
        .collect();
    assert_eq!(
        lines,
        [
            ("Ticket #1400".to_owned(), "6".to_owned()),
            (String::new(), "2".to_owned())
        ]
    );

    // Full day: Enter on the last line's hours goes to the next open day.
    press(&mut h, Modifiers::NONE, Key::Enter);
    press(&mut h, Modifiers::NONE, Key::Enter);
    assert!(focused(&h).starts_with("Add line"), "{}", focused(&h));
    assert_eq!(h.state().focused_day, Some(oct(15)));

    // Cmd+D copies the previous filled day (Wednesday) into Thursday.
    press(&mut h, Modifiers::COMMAND, Key::D);
    assert_eq!(focused(&h), "Description, Thursday line 1");
    assert_eq!(h.state().data().day_total(oct(15)), 8.0);

    // Cmd+Enter adds a line to the focused day (full days get none).
    press(&mut h, Modifiers::COMMAND, Key::Num5);
    press(&mut h, Modifiers::COMMAND, Key::Enter);
    assert_eq!(focused(&h), "Description, Friday line 1");
    press(&mut h, Modifiers::COMMAND, Key::Enter);
    assert_eq!(h.state().data().lines(oct(16)).count(), 1);
}

#[test]
fn shortcuts_move_between_weeks_and_months() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(dir.path());
    h.run();
    let at = |h: &Harness<'_, App>| (h.state().month.month, h.state().week);
    assert_eq!(at(&h), (10, 2));

    press(&mut h, Modifiers::COMMAND, Key::CloseBracket);
    assert_eq!(at(&h), (10, 3));
    press(
        &mut h,
        Modifiers::COMMAND | Modifiers::SHIFT,
        Key::OpenBracket,
    );
    assert_eq!(at(&h), (9, 0));
    press(&mut h, Modifiers::COMMAND, Key::OpenBracket);
    assert_eq!(at(&h), (8, 4), "last week of August");
    press(&mut h, Modifiers::COMMAND, Key::CloseCurlyBracket);
    assert_eq!(at(&h), (9, 0));

    // Cmd+1 does nothing in a week that starts after Monday (September 1 is a Tuesday).
    press(&mut h, Modifiers::COMMAND, Key::Num1);
    assert!(h.query_by(|n| n.is_focused()).is_none());

    press(&mut h, Modifiers::COMMAND, Key::T);
    assert_eq!(at(&h), (10, 2));
    assert_eq!(h.state().focused_day, Some(oct(14)));

    press(&mut h, Modifiers::COMMAND, Key::Num1);
    press(&mut h, Modifiers::COMMAND, Key::OpenBracket);
    press(&mut h, Modifiers::COMMAND, Key::J);
    assert_eq!(at(&h), (10, 2));
    assert_eq!(h.state().focused_day, Some(oct(14)));
}

#[test]
fn letter_keys_set_the_task_type_and_open_menus_block_shortcuts() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(dir.path());
    h.run();
    h.get_by_role_and_label(Role::ComboBox, "Task type, Monday line 1: Backend")
        .focus();
    h.run();
    press(&mut h, Modifiers::NONE, Key::S);
    let kind = |h: &Harness<'_, App>| h.state().data().lines(oct(12)).next().unwrap().kind;
    assert_eq!(kind(&h), Kind::Support);
    press(&mut h, Modifiers::NONE, Key::F);
    assert_eq!(kind(&h), Kind::Frontend);

    h.get_by_role_and_label(Role::ComboBox, "Task type, Monday line 1: Frontend")
        .click();
    h.run();
    press(&mut h, Modifiers::COMMAND, Key::CloseBracket);
    assert_eq!(h.state().week, 2);

    // In the open menu, a letter picks its option and closes the menu.
    press(&mut h, Modifiers::NONE, Key::B);
    assert_eq!(kind(&h), Kind::Backend);
    assert!(
        h.query_by_role_and_label(Role::RadioButton, "Support")
            .is_none()
    );
    assert_eq!(focused(&h), "Task type, Monday line 1: Backend");
}

#[test]
fn cmd_slash_toggles_the_shortcut_list() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(dir.path());
    h.run();
    press(&mut h, Modifiers::COMMAND, Key::Slash);
    h.get_by_label("Keyboard shortcuts");
    // Timesheet shortcuts are off while the list is open.
    press(&mut h, Modifiers::COMMAND, Key::CloseBracket);
    assert_eq!(h.state().week, 2);
    press(&mut h, Modifiers::NONE, Key::Escape);
    assert!(h.query_by_label("Keyboard shortcuts").is_none());
}

#[test]
fn cmd_comma_opens_settings_from_any_screen() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(dir.path());
    h.state_mut().view = View::PastMonths;
    h.run();
    press(&mut h, Modifiers::COMMAND, Key::Slash);
    press(&mut h, Modifiers::COMMAND, Key::Comma);
    assert!(h.state().settings_draft.is_some());
    assert!(
        !h.state().shortcuts_open,
        "settings replace the shortcut list"
    );
}
