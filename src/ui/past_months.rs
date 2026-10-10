//! Past months view: export, sent and approval tracking, signed copies.

use super::*;

impl App {
    /// Returns true when data changed.
    pub(super) fn past_months(&mut self, ui: &mut Ui) -> bool {
        let c = colors(ui);
        let today = self.today;
        let current = Month::of(today);
        let months = self.data().closed_months(current);
        let statuses: Vec<ApprovalStatus> = months
            .iter()
            .map(|&m| self.data().approval(m).status(today))
            .collect();
        let mut dirty = false;

        // Title row.
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = vec2(10.0, 6.0);
                ui.horizontal(|ui| {
                    let s = &self.store.data.settings;
                    text(
                        ui,
                        s.project_or_placeholder(),
                        sans(15.0, W::SemiBold),
                        c.ink,
                    );
                    text(
                        ui,
                        s.client_or_placeholder(),
                        sans(14.0, W::Regular),
                        c.muted,
                    );
                });
                text(ui, "Past months", sans(30.0, W::Bold), c.ink);
            });
            ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                let count =
                    |f: fn(&ApprovalStatus) -> bool| statuses.iter().filter(|s| f(s)).count();
                let counts = [
                    (
                        count(|s| matches!(s, ApprovalStatus::NotSent)),
                        "not sent",
                        c.chip,
                        c.ink2,
                    ),
                    (
                        count(|s| matches!(s, ApprovalStatus::Awaiting { .. })),
                        "awaiting signature",
                        c.accent_soft,
                        c.accent_strong,
                    ),
                    (
                        count(|s| matches!(s, ApprovalStatus::Approved)),
                        "approved",
                        c.ok_soft,
                        c.ok_fg,
                    ),
                ];
                for (n, label, bg, fg) in counts.into_iter().rev() {
                    pill(
                        ui,
                        &format!("{n} {label}"),
                        bg,
                        fg,
                        sans(14.0, W::SemiBold),
                        vec2(12.0, 8.0),
                        10,
                    );
                }
            });
        });

        if self.current_month_card(ui, current) {
            self.open_month(current);
            self.view = View::Timesheet;
        }

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(14.0, 6.0);
            text(ui, "Each month:", sans(14.0, W::SemiBold), c.ink);
            let steps = [
                "1. Export both PDFs",
                "2. Tick “Sent” when emailed to your manager",
                "3. Tick “Approved” when they reply",
                "4. Attach the signed PDF",
            ];
            for (i, step) in steps.into_iter().enumerate() {
                if i > 0 {
                    text(ui, "›", sans(14.0, W::Regular), c.ink2);
                }
                text(ui, step, sans(14.0, W::Regular), c.ink2);
            }
        });

        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 12.0;
            if months.is_empty() {
                text(
                    ui,
                    "No closed months yet. A month appears here once it ends.",
                    sans(15.0, W::Regular),
                    c.muted,
                );
            }
            for (&month, &status) in months.iter().zip(&statuses) {
                let mut approval = self.data().approval(month);
                let mut action = None;
                self.closed_month_card(ui, month, status, &mut approval, &mut action);
                if approval != self.data().approval(month) {
                    *self.store.data.approval_mut(month) = approval;
                    dirty = true;
                }
                dirty |= action.is_some_and(|a| self.month_action(month, a));
            }
        });
        dirty
    }

    /// Link card to the month being entered. Returns true when clicked.
    fn current_month_card(&self, ui: &mut Ui, month: Month) -> bool {
        let c = colors(ui);
        let data = self.data();
        let total = data.month_total(month);
        let target = data.target_hours(&month.weekdays());
        let (status, bg, fg) = progress_status(c, data, month);
        let frame = Frame::new()
            .fill(c.surface)
            .stroke(Stroke::new(1.0, c.border))
            .corner_radius(14)
            .inner_margin(Margin::symmetric(20, 16));
        let rect = frame
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    ui.horizontal(|ui| {
                        ui.set_min_width(150.0);
                        text(ui, month.label(), sans(17.0, W::Bold), c.ink);
                    });
                    pill(
                        ui,
                        status,
                        bg,
                        fg,
                        sans(13.0, W::SemiBold),
                        vec2(10.0, 4.0),
                        99,
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        text(
                            ui,
                            "Continue entering time",
                            sans(15.0, W::SemiBold),
                            c.accent,
                        );
                        ui.label(hours_job(
                            total,
                            mono(14.0, false),
                            c.ink,
                            Some((target, mono(14.0, false), c.ink)),
                            c.muted,
                        ));
                        ui.spacing_mut().item_spacing.x = 10.0;
                        bar(
                            ui,
                            ui.available_width(),
                            8.0,
                            &[((total / target) as f32, c.accent)],
                        );
                    });
                });
            })
            .response
            .rect;
        let response = ui.interact(rect, ui.id().with("current-month"), Sense::click());
        response.widget_info(|| {
            WidgetInfo::labeled(
                WidgetType::Link,
                true,
                format!("{}, continue entering time", month.label()),
            )
        });
        if response.hovered() {
            ui.painter().rect_stroke(
                rect,
                14,
                Stroke::new(1.0, c.hover_border),
                StrokeKind::Inside,
            );
        }
        focus_ring(ui, &response, rect, 14);
        response.on_hover_cursor(CursorIcon::PointingHand).clicked()
    }

    /// One closed month: totals, PDF exports, sent and approved ticks, signed copy.
    fn closed_month_card(
        &self,
        ui: &mut Ui,
        month: Month,
        status: ApprovalStatus,
        approval: &mut Approval,
        action: &mut Option<MonthAction>,
    ) {
        let c = colors(ui);
        let data = self.data();
        let today = self.today;
        let label = month.label();
        let can_export = !data.has_overfull_day(month);
        let (status_text, pill_bg, pill_fg, border) = match status {
            ApprovalStatus::NotSent => ("Not sent", c.chip, c.ink2, c.border),
            ApprovalStatus::Awaiting { late: true, .. } => {
                ("Follow up", c.warn_soft, c.warn, c.warn_border)
            }
            ApprovalStatus::Awaiting { .. } => (
                "Awaiting signature",
                c.accent_soft,
                c.accent_strong,
                c.border,
            ),
            ApprovalStatus::Approved if approval.signed_pdf.is_some() => {
                ("Approved · signed copy saved", c.ok_soft, c.ok_fg, c.border)
            }
            ApprovalStatus::Approved => ("Approved", c.ok_soft, c.ok_fg, c.border),
        };
        // Mockup flex bases, except the first column, widened to fit the "Approved · signed copy saved" pill.
        let columns = [
            (200.0, 1.0),
            (170.0, 1.0),
            (190.0, 1.0),
            (190.0, 1.0),
            (250.0, 1.4),
        ];
        card(ui, border, Margin::symmetric(20, 18), |ui| {
            flex_wrap(ui, &columns, vec2(28.0, 20.0), |ui, column| {
                ui.spacing_mut().item_spacing.y = 6.0;
                match column {
                    0 => {
                        text(ui, &label, sans(17.0, W::Bold), c.ink);
                        let mut job = hours_job(
                            data.month_total(month),
                            mono(14.0, true),
                            c.ink,
                            None,
                            c.muted,
                        );
                        let workdays = data.workdays(&month.weekdays()).len();
                        let days = format!(" · {workdays} workdays");
                        job.append(
                            &days,
                            0.0,
                            egui::TextFormat::simple(sans(14.0, W::Regular), c.ink2),
                        );
                        ui.label(job);
                        pill(
                            ui,
                            status_text,
                            pill_bg,
                            pill_fg,
                            sans(13.0, W::SemiBold),
                            vec2(10.0, 4.0),
                            99,
                        );
                    }
                    1 => {
                        caption(ui, "Generated PDFs");
                        ui.spacing_mut().item_spacing.y = 0.0;
                        for report in [Report::Detailed, Report::Summary] {
                            let mut b = Btn::base(c, report.title())
                                .icon(Icon::Download, 16.0)
                                .fg(c.accent)
                                .min(0.0, 32.0);
                            b.pad = 0.0;
                            b.left = true;
                            b.hover_fill = Color32::TRANSPARENT;
                            b.enabled = can_export;
                            b.a11y = Some(format!("Export {}, {label}", report.title()));
                            if b.show(ui).clicked() {
                                *action = Some(MonthAction::Export(report));
                            }
                        }
                        ui.spacing_mut().item_spacing.y = 6.0;
                        if !can_export {
                            text(
                                ui,
                                "Fix days over 8 h to export.",
                                sans(13.0, W::SemiBold),
                                c.danger,
                            );
                        }
                        self.export_note_ui(ui, month);
                    }
                    2 => {
                        let mut sent = approval.sent.is_some();
                        if checkbox(ui, &mut sent, "Sent to manager", c.accent, true).changed() {
                            approval.set_sent(sent, today);
                        }
                        match approval.sent.as_mut() {
                            Some(date) => {
                                date_field(
                                    ui,
                                    date,
                                    &format!("sent-{}", month.key()),
                                    &format!("Date sent, {label}"),
                                );
                            }
                            None => {
                                text(
                                    ui,
                                    "Tick after emailing both PDFs.",
                                    sans(13.0, W::Regular),
                                    c.muted,
                                );
                            }
                        }
                    }
                    3 => {
                        let mut approved = approval.approved.is_some();
                        let enabled = approval.sent.is_some();
                        if checkbox(ui, &mut approved, "Approved by manager", c.ok, enabled)
                            .changed()
                        {
                            approval.set_approved(approved, today);
                        }
                        if let Some(date) = approval.approved.as_mut() {
                            date_field(
                                ui,
                                date,
                                &format!("approved-{}", month.key()),
                                &format!("Date approved, {label}"),
                            );
                        } else if let ApprovalStatus::Awaiting { days, late } = status {
                            let ago = match days {
                                0 => "Sent today".to_owned(),
                                1 => "Sent 1 day ago".to_owned(),
                                n => format!("Sent {n} days ago"),
                            };
                            let (tail, color) = if late {
                                ("time to follow up", c.warn)
                            } else {
                                ("waiting for reply", c.ink2)
                            };
                            text(
                                ui,
                                format!("{ago} · {tail}"),
                                sans(13.0, W::SemiBold),
                                color,
                            );
                        }
                    }
                    _ => {
                        caption(ui, "Signed timesheet");
                        if let Some(name) = &approval.signed_pdf {
                            signed_file_row(ui, name, &label, action);
                        } else {
                            let mut b = Btn::base(c, "Attach signed PDF")
                                .icon(Icon::Paperclip, 18.0)
                                .fg(c.accent)
                                .font(sans(15.0, W::SemiBold))
                                .min(ui.available_width(), 44.0)
                                .a11y(format!("Attach signed PDF, {label}"));
                            b.dashed = true;
                            b.left = true;
                            b.pad = 14.0;
                            b.hover_fill = c.accent_tint;
                            if b.show(ui).clicked() {
                                *action = Some(MonthAction::Attach);
                            }
                            let hint = if status == ApprovalStatus::Approved {
                                "Optional · keeps a copy with this month."
                            } else {
                                "Attaching marks the month as sent and approved."
                            };
                            text(ui, hint, sans(13.0, W::Regular), c.muted);
                        }
                    }
                }
            });
        });
    }

    /// Runs a past-month action. Returns true when data changed.
    fn month_action(&mut self, month: Month, action: MonthAction) -> bool {
        let result = match action {
            MonthAction::Export(report) => {
                self.export(month, report);
                return false;
            }
            MonthAction::Attach => {
                let Some(path) = rfd::FileDialog::new()
                    .set_title(format!("Signed timesheet for {}", month.label()))
                    .add_filter("PDF", &["pdf"])
                    .pick_file()
                else {
                    return false;
                };
                self.store
                    .attach_signed_pdf(month, &path, self.today)
                    .map_err(|e| format!("Could not attach {}: {e}", path.display()))
            }
            MonthAction::Open => {
                let path = self.store.signed_pdf_path(month);
                if let Err(e) = open_path(&path) {
                    self.banner = Some(format!("Could not open {}: {e}", path.display()));
                }
                return false;
            }
            MonthAction::Remove => self
                .store
                .remove_signed_pdf(month)
                .map_err(|e| format!("Could not remove the signed PDF: {e}")),
        };
        match result {
            Ok(()) => true,
            Err(msg) => {
                self.banner = Some(msg);
                false
            }
        }
    }
}

/// Opens `path` with the system's default app.
fn open_path(path: &Path) -> std::io::Result<()> {
    if !path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file is missing",
        ));
    }
    let program = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    let mut child = std::process::Command::new(program).arg(path).spawn()?;
    std::thread::spawn(move || child.wait());
    Ok(())
}

/// Entry progress of a month, as (label, background, foreground).
fn progress_status(c: &Palette, data: &Data, month: Month) -> (&'static str, Color32, Color32) {
    if data.has_overfull_day(month) {
        ("Needs fixes", c.danger_soft, c.danger)
    } else if data.open_days(&month.weekdays()).is_empty() {
        ("Complete", c.ok_soft, c.ok_fg)
    } else if data.month_total(month) > 0.0 {
        ("In progress", c.chip, c.ink2)
    } else {
        ("Empty", c.chip, c.ink2)
    }
}

/// Saved signed copy: the name opens it, the cross removes it.
fn signed_file_row(ui: &mut Ui, name: &str, month_label: &str, action: &mut Option<MonthAction>) {
    let c = colors(ui);
    let frame = Frame::new()
        .fill(c.ok_tint)
        .stroke(Stroke::new(1.0, c.ok_border))
        .corner_radius(10)
        .inner_margin(Margin {
            left: 12,
            right: 6,
            top: 6,
            bottom: 6,
        });
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            // Full row height, so the icon centers on the 36 px name and remove button.
            let (rect, _) = ui.allocate_exact_size(vec2(18.0, 36.0), Sense::hover());
            Icon::FileCheck.paint(ui.painter(), rect.center(), 18.0, c.ok_fg, 2.0);
            // Leaves room for the item spacing before the name and the remove button.
            let name_w = (ui.available_width() - 8.0 - 8.0 - 36.0).max(0.0);
            ui.allocate_ui_with_layout(
                vec2(name_w, 36.0),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.set_width(name_w);
                    let rich = egui::RichText::new(name)
                        .font(sans(14.0, W::SemiBold))
                        .color(c.ok_fg);
                    let link = ui.add(egui::Label::new(rich).truncate().sense(Sense::click()));
                    link.widget_info(|| {
                        WidgetInfo::labeled(WidgetType::Link, true, format!("Open {name}"))
                    });
                    if link.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                        *action = Some(MonthAction::Open);
                    }
                },
            );
            let mut b = Btn::base(c, "")
                .icon(Icon::Close, 14.0)
                .fg(c.ink2)
                .min(36.0, 36.0)
                .a11y(format!("Remove signed PDF for {month_label}"));
            b.radius = 8;
            b.pad = 0.0;
            if b.show(ui).clicked() {
                *action = Some(MonthAction::Remove);
            }
        });
    });
}
