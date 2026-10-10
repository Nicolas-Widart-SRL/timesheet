//! Desktop UI replicating the mockups in `mockups/`: monthly entry and past months (sent & approval tracking).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{Datelike, NaiveDate};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, CursorIcon, FontData, FontDefinitions, FontFamily,
    FontId, Frame, Id, Layout, Margin, Painter, Pos2, Rect, Response, Sense, Shape, Stroke,
    StrokeKind, TextEdit, Ui, UiBuilder, Vec2, WidgetInfo, WidgetType, pos2, text::LayoutJob, vec2,
};

use crate::fonts;
use crate::holidays::Country;
use crate::model::{
    Appearance, Approval, ApprovalStatus, Data, DayOff, Kind, MAX_DAY_HOURS, Month, Settings,
    Store, day_range, fmt_hours, weekday_long, weekday_short,
};
use crate::pdf;

mod past_months;
#[cfg(test)]
mod screenshots;
mod settings;
mod theme;
mod timesheet;
mod widgets;

use theme::*;
use widgets::*;

// ---------------------------------------------------------------------------
// App

#[derive(PartialEq, Clone, Copy)]
enum View {
    Timesheet,
    PastMonths,
}

#[derive(Clone, Copy)]
enum Report {
    Detailed,
    Summary,
}

impl Report {
    fn title(self) -> &'static str {
        match self {
            Report::Detailed => "Detailed report",
            Report::Summary => "Daily summary",
        }
    }
}

enum Action {
    Add(NaiveDate),
    Copy { from: NaiveDate, to: NaiveDate },
    Remove(u64),
    SetDayOff(NaiveDate, Option<DayOff>),
}

/// Past-months actions that need dialogs or file access, run after drawing the card.
enum MonthAction {
    Export(Report),
    Attach,
    Open,
    Remove,
}

pub struct App {
    store: Store,
    data_path: PathBuf,
    month: Month,
    week: usize,
    view: View,
    today: NaiveDate,
    settings_draft: Option<Settings>,
    banner: Option<String>,
    /// Outcome of the last export, for the month it covered.
    export_note: Option<(Month, Result<String, String>)>,
    /// Outcome of the last data export or import, shown in Settings.
    backup_note: Option<Result<String, String>>,
}

fn data_path() -> PathBuf {
    directories::ProjectDirs::from("", "", "Timesheet")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
        .join("timesheet.json")
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::with(cc, data_path(), chrono::Local::now().date_naive())
    }

    fn with(cc: &eframe::CreationContext<'_>, data_path: PathBuf, today: NaiveDate) -> Self {
        setup_fonts(&cc.egui_ctx);
        setup_style(&cc.egui_ctx);
        let (store, banner) = Store::open(data_path.clone());
        cc.egui_ctx
            .set_theme(theme_preference(store.data.settings.appearance));
        let first_run = store.data.settings == Settings::default() && store.data.entries.is_empty();
        let mut app = App {
            settings_draft: first_run.then(|| store.data.settings.clone()),
            store,
            data_path,
            month: Month::of(today),
            week: 0,
            view: View::Timesheet,
            today,
            banner,
            export_note: None,
            backup_note: None,
        };
        app.open_month(Month::of(today));
        app
    }

    fn data(&self) -> &Data {
        &self.store.data
    }

    fn save(&mut self) {
        if let Err(e) = self.store.save() {
            self.banner = Some(format!(
                "Could not save to {}: {e}",
                self.data_path.display()
            ));
        }
    }

    /// Switches month and selects the week holding today (current month) or the first week.
    fn open_month(&mut self, month: Month) {
        self.month = month;
        self.export_note = None;
        self.week = month
            .weeks()
            .iter()
            .position(|w| w.last().is_some_and(|&d| d >= self.today))
            .filter(|_| month == Month::of(self.today))
            .unwrap_or(0);
    }

    fn export(&mut self, month: Month, report: Report) {
        let Some(mut path) = rfd::FileDialog::new()
            .set_title(format!("Export {}", report.title()))
            .set_file_name(pdf::file_name(month, report.title()))
            .add_filter("PDF", &["pdf"])
            .save_file()
        else {
            return;
        };
        if path
            .extension()
            .is_none_or(|e| !e.eq_ignore_ascii_case("pdf"))
        {
            path.set_extension("pdf");
        }
        let bytes = match report {
            Report::Detailed => pdf::detailed_report(self.data(), month),
            Report::Summary => pdf::daily_summary(self.data(), month),
        };
        let note = bytes
            .and_then(|b| {
                std::fs::write(&path, b)
                    .map_err(|e| format!("Could not write {}: {e}", path.display()))
            })
            .map(|()| format!("Saved {}", path.display()));
        self.export_note = Some((month, note));
    }

    fn export_note_ui(&self, ui: &mut Ui, month: Month) {
        match &self.export_note {
            Some((m, note)) if *m == month => note_ui(ui, note),
            _ => {}
        }
    }

    fn export_data(&mut self) {
        let Some(mut path) = rfd::FileDialog::new()
            .set_title("Export data")
            .set_file_name(format!("Timesheet backup {}.json", self.today))
            .add_filter("JSON", &["json"])
            .save_file()
        else {
            return;
        };
        if path
            .extension()
            .is_none_or(|e| !e.eq_ignore_ascii_case("json"))
        {
            path.set_extension("json");
        }
        let note = self
            .store
            .export_backup()
            .and_then(|b| std::fs::write(&path, b))
            .map(|()| format!("Saved {}", path.display()))
            .map_err(|e| format!("Could not export to {}: {e}", path.display()));
        self.backup_note = Some(note);
    }

    fn import_data(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Import data")
            .add_filter("JSON", &["json"])
            .pick_file()
        else {
            return;
        };
        let confirmed = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Warning)
            .set_title("Replace all data?")
            .set_description(format!(
                "All lines, settings, approvals and signed PDFs will be replaced by the content of {}. \
                 Your current data is first saved to a backup file next to the data file.",
                path.display()
            ))
            .set_buttons(rfd::MessageButtons::OkCancelCustom("Replace".into(), "Cancel".into()))
            .show();
        // The Windows message-box fallback reports a custom OK button as `Ok`.
        if !matches!(confirmed, rfd::MessageDialogResult::Ok)
            && confirmed != rfd::MessageDialogResult::Custom("Replace".into())
        {
            return;
        }
        let note = std::fs::read(&path)
            .and_then(|bytes| {
                self.store
                    .import_backup(&bytes, chrono::Local::now().naive_local())
            })
            .map(|previous| match previous {
                Some(previous) => format!(
                    "Imported {}. Previous data saved to {}",
                    path.display(),
                    previous.display()
                ),
                None => format!("Imported {}", path.display()),
            })
            .map_err(|e| format!("Could not import {}: {e}", path.display()));
        if note.is_ok() {
            self.settings_draft = Some(self.store.data.settings.clone());
            self.open_month(self.month);
        }
        self.backup_note = Some(note);
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        // Follows saved and imported settings; the new theme shows from the next frame.
        let theme = theme_preference(self.data().settings.appearance);
        if ui.ctx().options(|o| o.theme_preference) != theme {
            ui.ctx().set_theme(theme);
            ui.ctx().request_repaint();
        }
        let c = colors(ui);
        egui::Panel::top("top")
            .exact_size(64.0)
            .frame(Frame::new().fill(c.surface))
            .show(ui, |ui| self.top_bar(ui));

        let mut dirty = false;
        egui::CentralPanel::no_frame()
            .frame(Frame::new().fill(c.bg))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        centered(ui, 1280.0, 32.0, |ui| {
                            ui.add_space(32.0);
                            ui.spacing_mut().item_spacing.y = 20.0;
                            if let Some(msg) = self.banner.clone() {
                                self.banner_ui(ui, &msg);
                            }
                            match self.view {
                                View::Timesheet => dirty |= self.timesheet(ui),
                                View::PastMonths => dirty |= self.past_months(ui),
                            }
                            ui.add_space(32.0);
                        });
                    });
            });

        dirty |= self.settings_modal(ui.ctx());
        if dirty {
            self.save();
        }
    }
}

impl App {
    fn top_bar(&mut self, ui: &mut Ui) {
        let c = colors(ui);
        let full = ui.max_rect();
        ui.painter().hline(
            full.x_range(),
            full.bottom() - 0.5,
            Stroke::new(1.0, c.border),
        );
        centered(ui, 1280.0, 32.0, |ui| {
            ui.set_height(64.0);
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                let (rect, _) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::hover());
                paint_app_mark(ui.painter(), rect);
                text(ui, "Timesheet", sans(17.0, W::Bold), c.ink);
                ui.add_space(22.0);
                ui.spacing_mut().item_spacing.x = 4.0;
                for (view, label) in [
                    (View::Timesheet, "Timesheet"),
                    (View::PastMonths, "Past months"),
                ] {
                    let on = self.view == view;
                    let mut b = Btn::base(c, label).min(0.0, 40.0);
                    b.pad = 14.0;
                    b.radius = 8;
                    if on {
                        b = b.fg(c.accent_strong).font(sans(15.0, W::SemiBold));
                        b.fill = c.accent_soft;
                        b.hover_fill = c.accent_soft;
                    } else {
                        b = b.fg(c.ink2).font(sans(15.0, W::Regular));
                    }
                    if b.show(ui).clicked() {
                        self.view = view;
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let settings = &self.store.data.settings;
                    let (rect, response) = ui.allocate_exact_size(vec2(36.0, 36.0), Sense::click());
                    let name = settings.consultant_or_placeholder().to_owned();
                    response.widget_info(|| {
                        WidgetInfo::labeled(
                            WidgetType::Button,
                            true,
                            format!("Signed in as {name}. Edit settings"),
                        )
                    });
                    let fill = if response.hovered() { c.ink2 } else { c.ink };
                    ui.painter().circle_filled(rect.center(), 18.0, fill);
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        settings.initials(),
                        sans(13.0, W::SemiBold),
                        c.surface,
                    );
                    focus_ring(ui, &response, rect, 18);
                    if response
                        .on_hover_cursor(CursorIcon::PointingHand)
                        .on_hover_text("Settings")
                        .clicked()
                    {
                        self.settings_draft = Some(self.store.data.settings.clone());
                        self.backup_note = None;
                    }
                });
            });
        });
    }

    fn banner_ui(&mut self, ui: &mut Ui, msg: &str) {
        let c = colors(ui);
        Frame::new()
            .fill(c.danger_soft)
            .corner_radius(10)
            .inner_margin(Margin::symmetric(16, 12))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(msg)
                                .color(c.danger)
                                .font(sans(14.0, W::SemiBold)),
                        )
                        .wrap(),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if Btn::base(c, "")
                            .icon(Icon::Close, 16.0)
                            .a11y("Dismiss")
                            .min(32.0, 32.0)
                            .show(ui)
                            .clicked()
                        {
                            self.banner = None;
                        }
                    });
                });
            });
    }
}
