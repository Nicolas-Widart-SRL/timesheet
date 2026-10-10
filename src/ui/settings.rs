//! Settings dialog, also shown on first run.

use std::path::{MAIN_SEPARATOR, MAIN_SEPARATOR_STR};
use std::process::Command;

use super::*;

const WIDTH: f32 = 720.0;
/// Horizontal padding of every row in the dialog.
const PAD: i8 = 28;
/// Height of the header, footer and Data section, which never scroll.
const FIXED_HEIGHT: f32 = 420.0;

const REVEAL_LABEL: &str = if cfg!(target_os = "macos") {
    "Show in Finder"
} else if cfg!(windows) {
    "Show in Explorer"
} else {
    "Show in folder"
};

impl App {
    /// Returns true when settings were saved.
    pub(super) fn settings_modal(&mut self, ctx: &egui::Context) -> bool {
        let Some(draft) = self.settings_draft.as_mut() else {
            return false;
        };
        let mut save = false;
        let mut cancel = false;
        let mut export = false;
        let mut import = false;
        let mut reveal = false;
        let c = Palette::of(&ctx.global_style().visuals);
        let frame = Frame::new()
            .fill(c.surface)
            .corner_radius(14)
            .stroke(Stroke::new(1.0, c.border));
        let body_height = (ctx.content_rect().height() - FIXED_HEIGHT).max(160.0);
        let modal = egui::Modal::new(Id::new("settings"))
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_width(WIDTH);
                ui.spacing_mut().item_spacing.y = 0.0;

                Frame::new()
                    .inner_margin(Margin {
                        left: PAD,
                        right: 16,
                        top: 10,
                        bottom: 10,
                    })
                    .show(ui, |ui| {
                        ui.allocate_ui_with_layout(
                            vec2(ui.available_width(), 44.0),
                            Layout::right_to_left(Align::Center),
                            |ui| {
                                cancel = Btn::base(c, "")
                                    .icon(Icon::Close, 18.0)
                                    .fg(c.muted)
                                    .a11y("Close")
                                    .min(44.0, 44.0)
                                    .show(ui)
                                    .clicked();
                                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                    text(ui, "Settings", sans(20.0, W::Bold), c.ink);
                                });
                            },
                        );
                    });
                hline(ui, c.border);

                egui::ScrollArea::vertical()
                    .max_height(body_height)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        Frame::new()
                            .inner_margin(Margin::symmetric(PAD, 0))
                            .show(ui, |ui| {
                                section(
                                    ui,
                                    "Profile",
                                    "Shown in the app header and on both PDF exports.",
                                    |ui| {
                                        for (i, (label, value, hint)) in [
                                            ("Consultant name", &mut draft.consultant, "e.g. Jane Doe"),
                                            ("Client name", &mut draft.client, "e.g. Acme Corp"),
                                            ("Project name", &mut draft.project, "e.g. Website redesign"),
                                        ]
                                        .into_iter()
                                        .enumerate()
                                        {
                                            if i > 0 {
                                                ui.add_space(8.0);
                                            }
                                            text_field(ui, label, value, hint);
                                        }
                                    },
                                );
                                hline(ui, c.divider);
                                section(
                                    ui,
                                    "Holidays",
                                    "Marked as days off. Override one with \u{201c}Mark as workday\u{201d}.",
                                    |ui| {
                                        text(ui, "Country", sans(14.0, W::SemiBold), c.ink2);
                                        select_field(
                                            ui,
                                            "country",
                                            "Country",
                                            &mut draft.country,
                                            std::iter::once(None).chain(Country::ALL.map(Some)),
                                            |c| c.map_or("None", Country::label),
                                        );
                                    },
                                );
                                hline(ui, c.divider);
                                section(
                                    ui,
                                    "Appearance",
                                    "System follows your computer. PDFs always stay light.",
                                    |ui| {
                                        segmented(
                                            ui,
                                            &mut draft.appearance,
                                            &Appearance::ALL.map(|a| {
                                                let icon = match a {
                                                    Appearance::System => Icon::Monitor,
                                                    Appearance::Light => Icon::Sun,
                                                    Appearance::Dark => Icon::Moon,
                                                };
                                                (a, icon, a.label())
                                            }),
                                        );
                                    },
                                );
                            });
                    });

                hline(ui, c.border);
                Frame::new()
                    .inner_margin(Margin::symmetric(PAD, 16))
                    .show(ui, |ui| {
                        ui.allocate_ui_with_layout(
 vec2(ui.available_width(), 44.0),
 Layout::right_to_left(Align::Center),
 |ui| {
                            save = Btn::primary(c, "Save").min(96.0, 44.0).show(ui).clicked();
                            cancel |= Btn::outline(c, "Cancel").show(ui).clicked();
                        });
                    });

                // Data actions run at once, so they sit apart from the Save footer.
                hline(ui, c.border);
                Frame::new()
                    .fill(c.bg)
                    .corner_radius(CornerRadius {
                        nw: 0,
                        ne: 0,
                        sw: 13,
                        se: 13,
                    })
                    .inner_margin(Margin::symmetric(PAD, 0))
                    .show(ui, |ui| {
                        section(
                            ui,
                            "Data",
                            "Saved automatically. These actions run right away, without Save.",
                            |ui| {
                                ui.spacing_mut().item_spacing.y = 14.0;
                                reveal = path_row(ui, &self.data_path);
                                ui.horizontal(|ui| {
                                    export = Btn::outline(c, "Export data…").show(ui).clicked();
                                    import = Btn {
                                        hover_fill: c.danger_soft,
                                        ..Btn::outline(c, "Import data…").fg(c.danger)
                                    }
                                    .show(ui)
                                    .clicked();
                                });
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(
                                            "Import replaces all current data. \
                                             A backup is saved next to the data file first.",
                                        )
                                        .font(sans(13.0, W::Regular))
                                        .color(c.muted),
                                    )
                                    .wrap(),
                                );
                                if let Some(note) = &self.backup_note {
                                    note_ui(ui, note);
                                }
                            },
                        );
                    });
            });
        if reveal && let Err(e) = reveal_path(&self.data_path) {
            self.backup_note = Some(Err(format!(
                "Could not show {}: {e}",
                self.data_path.display()
            )));
        }
        if export {
            self.export_data();
        }
        if import {
            self.import_data();
            return false;
        }
        if save {
            let draft = self.settings_draft.take().unwrap_or_default();
            self.store.data.settings = Settings {
                consultant: draft.consultant.trim().to_owned(),
                client: draft.client.trim().to_owned(),
                project: draft.project.trim().to_owned(),
                country: draft.country,
                appearance: draft.appearance,
            };
            return true;
        }
        if cancel || modal.should_close() {
            self.settings_draft = None;
        }
        false
    }
}

/// One settings group: title and help in a 200 px column, controls on the right, 24 px above and below.
fn section(ui: &mut Ui, title: &str, help: &str, add: impl FnOnce(&mut Ui)) {
    const LEFT: f32 = 200.0;
    let c = colors(ui);
    ui.add_space(24.0);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 32.0;
        ui.allocate_ui_with_layout(vec2(LEFT, 0.0), Layout::top_down(Align::Min), |ui| {
            ui.set_width(LEFT);
            ui.spacing_mut().item_spacing.y = 4.0;
            text(ui, title, sans(16.0, W::SemiBold), c.ink);
            ui.add(
                egui::Label::new(
                    egui::RichText::new(help)
                        .font(sans(13.0, W::Regular))
                        .color(c.muted),
                )
                .wrap(),
            );
        });
        let w = ui.available_width();
        ui.allocate_ui_with_layout(vec2(w, 0.0), Layout::top_down(Align::Min), |ui| {
            ui.set_width(w);
            ui.spacing_mut().item_spacing = vec2(8.0, 6.0);
            add(ui);
        });
    });
    ui.add_space(24.0);
}

fn text_field(ui: &mut Ui, label: &str, value: &mut String, hint: &str) {
    let c = colors(ui);
    let label = text(ui, label, sans(14.0, W::SemiBold), c.ink2);
    ui.add(
        TextEdit::singleline(value)
            .hint_text(hint)
            .font(sans(15.0, W::Regular))
            .text_color(c.ink)
            .margin(Margin::symmetric(12, 0))
            .vertical_align(Align::Center)
            .min_size(vec2(0.0, 44.0))
            .desired_width(f32::INFINITY),
    )
    .labelled_by(label.id);
}

/// Data file path on one line, shortened in the middle, with a button that reveals it.
/// Returns true when the button was clicked.
fn path_row(ui: &mut Ui, path: &Path) -> bool {
    let c = colors(ui);
    let mut clicked = false;
    Frame::new()
        .fill(c.surface)
        .stroke(Stroke::new(1.0, c.border))
        .corner_radius(10)
        .inner_margin(Margin {
            left: 12,
            right: 4,
            top: 4,
            bottom: 4,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.allocate_ui_with_layout(
                vec2(ui.available_width(), 44.0),
                Layout::right_to_left(Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    clicked = Btn {
                        pad: 12.0,
                        enabled: path.exists(),
                        ..Btn::base(c, REVEAL_LABEL)
                            .icon(Icon::Folder, 16.0)
                            .fg(c.accent)
                            .font(sans(14.0, W::Medium))
                    }
                    .show(ui)
                    .clicked();
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        let full = path.display().to_string();
                        let font = mono(12.0, false);
                        let width = ui.available_width();
                        let short = shorten_path(&full, |s| {
                            ui.painter()
                                .layout_no_wrap(s.to_owned(), font.clone(), c.ink2)
                                .size()
                                .x
                                <= width
                        });
                        ui.add(egui::Label::new(
                            egui::RichText::new(short).font(font).color(c.ink2),
                        ))
                        .on_hover_text(full);
                    });
                },
            );
        });
    clicked
}

/// Replaces folders in the middle of `path` with "…" until `fits` accepts it,
/// e.g. "/var/folders/37/…/timesheet.json". Keeps the first folder and the file name, so it may still not fit.
fn shorten_path(path: &str, fits: impl Fn(&str) -> bool) -> String {
    let parts: Vec<&str> = path.split(MAIN_SEPARATOR).collect();
    let n = parts.len();
    if fits(path) || n < 4 {
        return path.to_owned();
    }
    // parts[..head] and parts[tail..] stay on both sides of the ellipsis.
    let (mut head, mut tail) = (n / 2, n / 2 + 1);
    loop {
        let short = [
            parts[..head].join(MAIN_SEPARATOR_STR),
            "…".into(),
            parts[tail..].join(MAIN_SEPARATOR_STR),
        ]
        .join(MAIN_SEPARATOR_STR);
        if fits(&short) || (head <= 2 && tail >= n - 1) {
            return short;
        }
        if head > 2 && (head >= n - tail || tail >= n - 1) {
            head -= 1;
        } else {
            tail += 1;
        }
    }
}

/// Shows `path` selected in Finder or Explorer; other systems open its folder.
fn reveal_path(path: &Path) -> std::io::Result<()> {
    let mut child = reveal_command(path).spawn()?;
    std::thread::spawn(move || child.wait());
    Ok(())
}

#[cfg(target_os = "macos")]
fn reveal_command(path: &Path) -> Command {
    let mut cmd = Command::new("open");
    cmd.arg("-R").arg(path);
    cmd
}

#[cfg(windows)]
fn reveal_command(path: &Path) -> Command {
    use std::os::windows::process::CommandExt;
    let mut cmd = Command::new("explorer");
    // Explorer parses its own command line and rejects a quoted "/select,…" argument.
    cmd.raw_arg(format!("/select,\"{}\"", path.display()));
    cmd
}

#[cfg(not(any(target_os = "macos", windows)))]
fn reveal_command(path: &Path) -> Command {
    let mut cmd = Command::new("xdg-open");
    cmd.arg(path.parent().unwrap_or(path));
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    fn native(path: &str) -> String {
        path.replace('/', MAIN_SEPARATOR_STR)
    }

    #[test]
    fn shorten_path_drops_middle_folders_until_it_fits() {
        let path = native("/var/folders/37/y77c/T/timesheet/timesheet.json");
        let short = |max: usize| shorten_path(&path, |s| s.chars().count() <= max);
        assert_eq!(short(100), path);
        assert_eq!(
            short(44),
            native("/var/folders/37/…/T/timesheet/timesheet.json")
        );
        assert_eq!(
            short(41),
            native("/var/folders/…/T/timesheet/timesheet.json")
        );
        assert_eq!(short(31), native("/var/…/timesheet/timesheet.json"));
        assert_eq!(short(30), native("/var/…/timesheet.json"));
        // Never drops the first folder or the file name.
        assert_eq!(short(5), native("/var/…/timesheet.json"));
    }

    #[test]
    fn shorten_path_keeps_short_paths() {
        let path = native("/a/b.json");
        assert_eq!(shorten_path(&path, |_| false), path);
    }
}
