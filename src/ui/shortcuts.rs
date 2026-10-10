//! Keyboard shortcuts: their keys, the timesheet commands they trigger and the overlay that lists them.
//! `Cmd` is Command on macOS and Ctrl on Windows, so shortcuts also work while a text field has focus.

use egui::{InputState, KeyboardShortcut};

use super::*;

const fn cmd(key: Key) -> KeyboardShortcut {
    KeyboardShortcut::new(Modifiers::COMMAND, key)
}

const fn cmd_shift(key: Key) -> KeyboardShortcut {
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), key)
}

/// Monday to Friday of the shown week.
pub(super) const DAYS: [KeyboardShortcut; 5] = [
    cmd(Key::Num1),
    cmd(Key::Num2),
    cmd(Key::Num3),
    cmd(Key::Num4),
    cmd(Key::Num5),
];
pub(super) const ADD_LINE: KeyboardShortcut = cmd(Key::Enter);
pub(super) const COPY_DAY: KeyboardShortcut = cmd(Key::D);
pub(super) const TODAY: KeyboardShortcut = cmd(Key::T);
pub(super) const PREV_WEEK: KeyboardShortcut = cmd(Key::OpenBracket);
pub(super) const NEXT_WEEK: KeyboardShortcut = cmd(Key::CloseBracket);
pub(super) const PREV_MONTH: KeyboardShortcut = cmd_shift(Key::OpenBracket);
pub(super) const NEXT_MONTH: KeyboardShortcut = cmd_shift(Key::CloseBracket);
pub(super) const NEXT_OPEN_DAY: KeyboardShortcut = cmd(Key::J);
pub(super) const SHOW_LIST: KeyboardShortcut = cmd(Key::Slash);

const ENTER: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::Enter);

/// Sets a line's task type while its task-type button has focus.
pub(super) fn kind_key(kind: Kind) -> Key {
    match kind {
        Kind::Frontend => Key::F,
        Kind::Backend => Key::B,
        Kind::Support => Key::S,
    }
}

/// A timesheet shortcut pressed this frame.
pub(super) enum Command {
    /// Weekday from Monday (0) to Friday (4).
    Day(u32),
    AddLine,
    CopyDay,
    Today,
    PrevWeek,
    NextWeek,
    PrevMonth,
    NextMonth,
    NextOpenDay,
}

/// Consumes the first timesheet shortcut pressed this frame.
pub(super) fn read(i: &mut InputState) -> Option<Command> {
    // Months first: `consume_shortcut` ignores an extra Shift, so Cmd+Shift+[ also matches Cmd+[.
    // With Shift, some layouts report `{` and `}` instead of the brackets.
    if i.consume_shortcut(&PREV_MONTH) || i.consume_shortcut(&cmd(Key::OpenCurlyBracket)) {
        return Some(Command::PrevMonth);
    }
    if i.consume_shortcut(&NEXT_MONTH) || i.consume_shortcut(&cmd(Key::CloseCurlyBracket)) {
        return Some(Command::NextMonth);
    }
    if let Some(n) = DAYS.iter().position(|s| i.consume_shortcut(s)) {
        return Some(Command::Day(n as u32));
    }
    [
        (ADD_LINE, Command::AddLine),
        (COPY_DAY, Command::CopyDay),
        (TODAY, Command::Today),
        (PREV_WEEK, Command::PrevWeek),
        (NEXT_WEEK, Command::NextWeek),
        (NEXT_OPEN_DAY, Command::NextOpenDay),
    ]
    .into_iter()
    .find_map(|(s, command)| i.consume_shortcut(&s).then_some(command))
}

/// "Cmd+Shift+]" on macOS, "Ctrl+Shift+]" on Windows. Unlike `Context::format_shortcut`, keeps
/// symbol keys as symbols ("[" rather than "OpenBracket") and needs no ⌘ glyph in the font.
pub(super) fn format(ctx: &egui::Context, shortcut: &KeyboardShortcut) -> String {
    let mut s = egui::ModifierNames::NAMES.format(&shortcut.modifiers, ctx.os().is_mac());
    if !s.is_empty() {
        s.push('+');
    }
    s + shortcut.logical_key.symbol_or_name()
}

/// Tooltip text for a button with a shortcut: "Next month (Cmd+Shift+])".
pub(super) fn hint(ui: &Ui, label: &str, shortcut: &KeyboardShortcut) -> String {
    format!("{label} ({})", format(ui.ctx(), shortcut))
}

impl App {
    /// Overlay listing every shortcut, toggled with [`SHOW_LIST`].
    pub(super) fn shortcuts_modal(&mut self, ctx: &egui::Context) {
        if !self.shortcuts_open {
            return;
        }
        let c = Palette::of(&ctx.global_style().visuals);
        let key = |s: &KeyboardShortcut| format(ctx, s);
        let days = format!("{} – {}", key(&DAYS[0]), key(&DAYS[4]));
        let enter = key(&ENTER);
        let kinds = Kind::ALL.map(|k| kind_key(k).name()).join(" / ");
        let sections = [
            (
                "Daily entry",
                vec![
                    (days, "Jump to Monday … Friday of the shown week"),
                    (key(&ADD_LINE), "Add a line to the current day"),
                    (enter.clone(), "Description: go to its hours"),
                    (enter, "Hours: next line, a new line or the next open day"),
                    (
                        key(&COPY_DAY),
                        "Copy the previous filled day into an empty day",
                    ),
                    (key(&TODAY), "Go to today"),
                    ("Esc".to_owned(), "Leave the focused field"),
                ],
            ),
            (
                "Navigation",
                vec![
                    (
                        format!("{} / {}", key(&PREV_WEEK), key(&NEXT_WEEK)),
                        "Previous / next week",
                    ),
                    (
                        format!("{} / {}", key(&PREV_MONTH), key(&NEXT_MONTH)),
                        "Previous / next month",
                    ),
                    (key(&NEXT_OPEN_DAY), "Next open day"),
                    (kinds, "On a task-type button: Frontend / Backend / Support"),
                    (key(&SHOW_LIST), "Show or hide this list"),
                ],
            ),
        ];

        let frame = Frame::new()
            .fill(c.surface)
            .corner_radius(14)
            .stroke(Stroke::new(1.0, c.border));
        let mut close = false;
        let modal = egui::Modal::new(Id::new("shortcuts"))
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_width(560.0);
                ui.spacing_mut().item_spacing.y = 0.0;
                Frame::new()
                    .inner_margin(Margin {
                        left: 24,
                        right: 16,
                        top: 10,
                        bottom: 10,
                    })
                    .show(ui, |ui| {
                        ui.allocate_ui_with_layout(
                            vec2(ui.available_width(), 44.0),
                            Layout::right_to_left(Align::Center),
                            |ui| {
                                close = Btn::base(c, "")
                                    .icon(Icon::Close, 18.0)
                                    .fg(c.muted)
                                    .a11y("Close")
                                    .min(44.0, 44.0)
                                    .show(ui)
                                    .clicked();
                                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                    text(ui, "Keyboard shortcuts", sans(20.0, W::Bold), c.ink);
                                });
                            },
                        );
                    });
                hline(ui, c.border);
                Frame::new()
                    .inner_margin(Margin::symmetric(24, 20))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        for (k, (title, rows)) in sections.iter().enumerate() {
                            if k > 0 {
                                ui.add_space(12.0);
                            }
                            caption(ui, title);
                            for (keys, action) in rows {
                                ui.horizontal(|ui| {
                                    text(ui, *action, sans(15.0, W::Regular), c.ink);
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        pill(
                                            ui,
                                            keys,
                                            c.chip,
                                            c.ink2,
                                            mono(13.0, false),
                                            vec2(8.0, 3.0),
                                            6,
                                        );
                                    });
                                });
                            }
                        }
                    });
            });
        if close || modal.should_close() {
            self.shortcuts_open = false;
        }
    }
}
