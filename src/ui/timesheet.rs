//! Monthly entry view: week chips, day cards and the month sidebar.

use super::*;

impl App {
    /// Returns true when data changed.
    pub(super) fn timesheet(&mut self, ui: &mut Ui) -> bool {
        let c = colors(ui);
        let mut dirty = self.timesheet_shortcuts(ui);
        let month = self.month;
        let weeks = month.weeks();
        self.week = self.week.min(weeks.len().saturating_sub(1));
        let mut focus = self.focus.take();

        // Title row.
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = vec2(10.0, 8.0);
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
                    pill(
                        ui,
                        "Draft · sent monthly",
                        c.chip,
                        c.ink2,
                        sans(13.0, W::SemiBold),
                        vec2(10.0, 3.0),
                        99,
                    );
                });
                ui.horizontal(|ui| {
                    if Btn::outline(c, "")
                        .icon(Icon::ChevronLeft, 18.0)
                        .a11y("Previous month")
                        .min(44.0, 44.0)
                        .show(ui)
                        .on_hover_text(shortcuts::hint(
                            ui,
                            "Previous month",
                            &shortcuts::PREV_MONTH,
                        ))
                        .clicked()
                    {
                        self.open_month(month.prev());
                    }
                    ui.add_space(4.0);
                    text(ui, month.label(), sans(30.0, W::Bold), c.ink);
                    ui.add_space(4.0);
                    if Btn::outline(c, "")
                        .icon(Icon::ChevronRight, 18.0)
                        .a11y("Next month")
                        .min(44.0, 44.0)
                        .show(ui)
                        .on_hover_text(shortcuts::hint(ui, "Next month", &shortcuts::NEXT_MONTH))
                        .clicked()
                    {
                        self.open_month(month.next());
                    }
                });
            });
            ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                text(
                    ui,
                    "Weekdays only · max 8 h per day",
                    sans(14.0, W::Regular),
                    c.muted,
                );
                for kind in Kind::ALL.iter().rev() {
                    // Inside a right-to-left layout: the label is added first so the swatch sits on its left.
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        text(ui, kind.label(), sans(14.0, W::Regular), c.ink2);
                        swatch(ui, c.kind(*kind));
                    });
                }
            });
        });

        self.week_chips(ui, &weeks);

        let wide = ui.available_width() >= 1000.0;
        let side_w = 340.0;
        let gap = 24.0;
        let main_w = if wide {
            ui.available_width() - side_w - gap
        } else {
            ui.available_width()
        };
        let mut days_ui = |ui: &mut Ui, this: &mut Self| {
            ui.set_width(main_w);
            ui.spacing_mut().item_spacing.y = 16.0;
            let mut actions = Vec::new();
            let mut cards = Vec::new();
            for &date in &weeks[this.week] {
                let card = ui.scope(|ui| {
                    let data = &mut this.store.data;
                    day_card(ui, data, &mut this.tickets, date, &mut focus, &mut actions)
                });
                dirty |= card.inner;
                cards.push((date, card.response.rect));
            }
            let focused = ui.ctx().memory(|m| m.focused());
            let focused = focused.and_then(|id| ui.ctx().read_response(id));
            this.focused_day = focused.and_then(|r| {
                let card = cards.iter().find(|(_, rect)| rect.contains_rect(r.rect));
                card.map(|&(date, _)| date)
            });
            // Enter after a full day's last line can lead to a day in another week.
            if let Some(Focus::Day(date)) = focus {
                this.jump_to(date);
            }
            dirty |= !actions.is_empty();
            for action in actions {
                let data = &mut this.store.data;
                match action {
                    Action::Add(d) => this.focus = Some(Focus::Desc(data.add_line(d))),
                    Action::Copy { from, to } => {
                        data.copy_day(from, to);
                        this.focus = Some(Focus::Day(to));
                    }
                    Action::Remove(id) => data.remove_line(id),
                    Action::SetDayOff(d, off) => {
                        data.set_day_off(d, off);
                        this.focus = Some(Focus::Day(d));
                    }
                }
            }
        };
        if wide {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.vertical(|ui| days_ui(ui, self));
                ui.vertical(|ui| {
                    ui.set_width(side_w);
                    self.sidebar(ui);
                });
            });
        } else {
            ui.vertical(|ui| days_ui(ui, self));
            self.sidebar(ui);
        }
        dirty
    }

    /// Applies the timesheet shortcut pressed this frame, if any. Returns true when data changed.
    fn timesheet_shortcuts(&mut self, ui: &Ui) -> bool {
        if self.settings_draft.is_some()
            || self.shortcuts_open
            || egui::Popup::is_any_open(ui.ctx())
        {
            return false;
        }
        let Some(command) = ui.input_mut(shortcuts::read) else {
            return false;
        };
        let weeks = self.month.weeks();
        self.week = self.week.min(weeks.len() - 1);
        let week = &weeks[self.week];
        let data = &mut self.store.data;
        // The day holding the focused widget, else a default day of the shown week.
        let current = self
            .focused_day
            .filter(|d| week.contains(d))
            .or_else(|| data.default_day(week, self.today));
        match command {
            shortcuts::Command::Day(n) => {
                if let Some(&d) = week
                    .iter()
                    .find(|d| d.weekday().num_days_from_monday() == n)
                {
                    self.focus = Some(Focus::Day(d));
                }
            }
            shortcuts::Command::AddLine => {
                if let Some(d) = current.filter(|&d| data.can_add_line(d)) {
                    self.focus = Some(Focus::Desc(data.add_line(d)));
                    return true;
                }
            }
            shortcuts::Command::CopyDay => {
                if let Some((from, to)) = current.and_then(|d| Some((data.copy_source(d)?, d))) {
                    data.copy_day(from, to);
                    self.focus = Some(Focus::Day(to));
                    return true;
                }
            }
            shortcuts::Command::Today => {
                self.open_month(Month::of(self.today));
                let week = &self.month.weeks()[self.week];
                self.focus = self.data().default_day(week, self.today).map(Focus::Day);
            }
            shortcuts::Command::PrevWeek if self.week > 0 => self.week -= 1,
            shortcuts::Command::PrevWeek => {
                self.open_month(self.month.prev());
                self.week = self.month.weeks().len() - 1;
            }
            shortcuts::Command::NextWeek if self.week + 1 < weeks.len() => self.week += 1,
            shortcuts::Command::NextWeek => {
                self.open_month(self.month.next());
                self.week = 0;
            }
            shortcuts::Command::PrevMonth => self.open_month(self.month.prev()),
            shortcuts::Command::NextMonth => self.open_month(self.month.next()),
            shortcuts::Command::NextOpenDay => {
                if let Some(&d) = data.open_days(&self.month.weekdays()).first() {
                    self.jump_to(d);
                }
            }
        }
        false
    }

    fn week_chips(&mut self, ui: &mut Ui, weeks: &[Vec<NaiveDate>]) {
        let gap = 10.0;
        let width = ui.available_width();
        let cols = (((width + gap) / 160.0).floor() as usize).clamp(1, weeks.len().max(1));
        let chip_w = (width - gap * (cols - 1) as f32) / cols as f32;
        let month = self.month;
        for row in weeks
            .chunks(cols)
            .enumerate()
            .map(|(r, c)| (r * cols, c))
            .collect::<Vec<_>>()
        {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (k, week) in row.1.iter().enumerate() {
                    let index = row.0 + k;
                    if week_chip(ui, self.data(), month, week, index == self.week, chip_w).clicked()
                    {
                        self.week = index;
                    }
                }
            });
        }
        ui.add_space(-10.0);
    }

    fn sidebar(&mut self, ui: &mut Ui) {
        let c = colors(ui);
        let month = self.month;
        let data = self.data();
        let days = data.workdays(&month.weekdays());
        let open = data.open_days(&days);
        let any_over = data.has_overfull_day(month);
        let target = data.target_hours(&days);
        let total = data.month_total(month);
        let by_kind: Vec<(Kind, f64)> = Kind::ALL
            .iter()
            .map(|&k| (k, data.kind_total(month, k)))
            .collect();
        ui.spacing_mut().item_spacing.y = 16.0;

        card(ui, c.border, Margin::same(20), |ui| {
            ui.spacing_mut().item_spacing.y = 16.0;
            ui.horizontal(|ui| {
                text(ui, month.name(), sans(17.0, W::Bold), c.ink);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(hours_job(
                        total,
                        mono(22.0, true),
                        c.ink,
                        Some((target, mono(14.0, false), c.muted)),
                        c.muted,
                    ));
                });
            });
            let segments: Vec<(f32, Color32)> = by_kind
                .iter()
                .map(|&(k, h)| ((h / target) as f32, c.kind(k)))
                .collect();
            bar(ui, ui.available_width(), 10.0, &segments);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                for &(k, h) in &by_kind {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        swatch(ui, c.kind(k));
                        text(ui, k.label(), sans(15.0, W::Regular), c.ink);
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(hours_job(h, mono(15.0, false), c.ink, None, c.muted));
                        });
                    });
                }
            });
            hline(ui, c.divider);
            ui.add_space(-2.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
                let mut job = LayoutJob::default();
                job.append(
                    &format!("{} of {}", days.len() - open.len(), days.len()),
                    0.0,
                    egui::TextFormat::simple(sans(14.0, W::Bold), c.ink),
                );
                job.append(
                    " workdays complete",
                    0.0,
                    egui::TextFormat::simple(sans(14.0, W::Regular), c.ink2),
                );
                ui.add(egui::Label::new(job).extend());
                if let Some(&first_open) = open.first() {
                    let label = format!(
                        "Next open day: {} {}",
                        weekday_short(first_open),
                        first_open.day()
                    );
                    let mut b = Btn::outline(c, &label)
                        .fg(c.accent)
                        .font(sans(15.0, W::SemiBold))
                        .min(0.0, 40.0);
                    b.pad = 12.0;
                    let tip = shortcuts::hint(ui, "Next open day", &shortcuts::NEXT_OPEN_DAY);
                    if b.show(ui).on_hover_text(tip).clicked() {
                        self.jump_to(first_open);
                    }
                }
            });
        });

        let mut export = None;
        card(ui, c.border, Margin::same(20), |ui| {
            ui.spacing_mut().item_spacing.y = 12.0;
            text(
                ui,
                format!("Export {}", month.name()),
                sans(17.0, W::Bold),
                c.ink,
            );
            if any_over {
                text(
                    ui,
                    "Fix days over 8 h before exporting.",
                    sans(14.0, W::SemiBold),
                    c.danger,
                );
            }
            if !open.is_empty() {
                let msg = format!(
                    "{} workdays still under 8 h. You can export a draft anytime.",
                    open.len()
                );
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(msg)
                            .font(sans(14.0, W::Regular))
                            .color(c.warn),
                    )
                    .wrap(),
                );
            }
            let full = ui.available_width();
            for (report, primary) in [(Report::Detailed, true), (Report::Summary, false)] {
                let label = format!("{} (PDF)", report.title());
                let mut b = if primary {
                    Btn::primary(c, &label)
                } else {
                    Btn::outline(c, &label).font(sans(15.0, W::SemiBold))
                };
                b = b.icon(Icon::Download, 18.0).min(full, 48.0);
                b.left = true;
                b.enabled = !any_over;
                if b.show(ui).clicked() {
                    export = Some(report);
                }
            }
            let note = format!(
                "Both PDFs cover {}, weekdays only.",
                month.period_label().rsplit_once(',').map_or("", |p| p.0)
            );
            text(ui, note, sans(13.0, W::Regular), c.muted);
            self.export_note_ui(ui, month);
        });
        if let Some(report) = export {
            self.export(month, report);
        }
    }
}

// ---------------------------------------------------------------------------
// Week chip and day card

fn week_chip(
    ui: &mut Ui,
    data: &Data,
    month: Month,
    week: &[NaiveDate],
    selected: bool,
    width: f32,
) -> Response {
    let c = colors(ui);
    let total = data.days_total(week);
    let target = data.target_hours(week);
    let over = week.iter().any(|&d| data.day_total(d) > MAX_DAY_HOURS);
    let (status, status_color, bar_color) = if over {
        ("Over", c.danger, c.danger)
    } else if total >= target {
        ("Done", c.ok_fg, c.ok)
    } else if total > 0.0 {
        ("In progress", c.muted, c.accent)
    } else {
        ("Empty", c.muted, c.accent)
    };
    let label = format!("{} {}", month.short_name(), day_range(week, "–"));

    let (rect, response) = ui.allocate_exact_size(vec2(width, 64.0), Sense::click());
    response.widget_info(|| {
        WidgetInfo::selected(
            WidgetType::Button,
            true,
            selected,
            format!("{label}, {status}"),
        )
    });
    let p = ui.painter();
    let (fill, stroke) = if selected {
        (c.accent_tint, Stroke::new(2.0, c.accent))
    } else if response.hovered() {
        (c.surface, Stroke::new(1.0, c.hover_border))
    } else {
        (c.surface, Stroke::new(1.0, c.border))
    };
    p.rect_filled(rect, 12, fill);
    p.rect_stroke(rect, 12, stroke, StrokeKind::Inside);
    focus_ring(ui, &response, rect, 12);
    let inner = rect.shrink2(vec2(14.0, 10.0));
    p.text(
        inner.left_top(),
        Align2::LEFT_TOP,
        &label,
        sans(15.0, W::SemiBold),
        c.ink,
    );
    p.text(
        pos2(inner.right(), inner.top() + 2.0),
        Align2::RIGHT_TOP,
        status,
        sans(12.0, W::SemiBold),
        status_color,
    );
    let numbers = format!("{}/{}", fmt_hours(total), fmt_hours(target));
    let numbers_rect = p.text(
        inner.right_bottom(),
        Align2::RIGHT_BOTTOM,
        numbers,
        mono(13.0, false),
        c.ink2,
    );
    let bar_rect = Rect::from_min_max(
        pos2(inner.left(), numbers_rect.center().y - 3.0),
        pos2(numbers_rect.left() - 8.0, numbers_rect.center().y + 3.0),
    );
    paint_bar(
        p,
        bar_rect,
        c.border,
        &[((total / target).min(1.0) as f32, bar_color)],
    );
    response.on_hover_cursor(CursorIcon::PointingHand)
}

/// Focuses `response` when it is the pending `target`.
fn take_focus(response: &Response, pending: &mut Option<Focus>, target: Focus) {
    if *pending == Some(target) {
        response.request_focus();
        response.scroll_to_me(None);
        *pending = None;
    }
}

/// One weekday card with its lines. Edits apply directly; structural changes are queued in `actions`.
/// Takes the pending `focus` when it targets a widget of this card. Returns true when a line was edited.
fn day_card(
    ui: &mut Ui,
    data: &mut Data,
    tickets: &mut TicketPicker,
    date: NaiveDate,
    focus: &mut Option<Focus>,
    actions: &mut Vec<Action>,
) -> bool {
    let c = colors(ui);
    if let Some(label) = data.day_off_label(date) {
        day_off_card(ui, date, &label, focus, actions);
        return false;
    }
    let total = data.day_total(date);
    let left = MAX_DAY_HOURS - total;
    let over = total > MAX_DAY_HOURS;
    let next_hint = format!(
        "{} {} h",
        data.next_kind(date).label(),
        fmt_hours(left.max(0.0))
    );
    let prev = data.copy_source(date);
    // Where Enter in the last line's hours goes: a new line, else the next open day.
    let can_add = data.can_add_line(date);
    let next_day = data.next_open_day(date, &Month::of(date).weekdays());
    let indexes: Vec<usize> = data
        .entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.date == date)
        .map(|(i, _)| i)
        .collect();
    let segments: Vec<(f32, Color32)> = Kind::ALL
        .iter()
        .map(|&k| {
            (
                (data.day_kind_total(date, k) / MAX_DAY_HOURS) as f32,
                c.kind(k),
            )
        })
        .collect();
    let (status, pill_bg, pill_fg) = if over {
        ("Over 8 h".to_owned(), c.danger_soft, c.danger)
    } else if left <= 0.0 {
        ("Complete".to_owned(), c.ok_soft, c.ok_fg)
    } else {
        (format!("{} h left", fmt_hours(left)), c.chip, c.ink2)
    };
    let day_name = weekday_long(date);
    let mut dirty = false;

    let border = if over { c.danger } else { c.border };
    card(ui, border, Margin::ZERO, |ui| {
        ui.spacing_mut().item_spacing = vec2(12.0, 0.0);
        // Header.
        Frame::new()
            .inner_margin(Margin::symmetric(20, 14))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    day_heading(ui, date);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        pill(
                            ui,
                            &status,
                            pill_bg,
                            pill_fg,
                            sans(13.0, W::SemiBold),
                            vec2(10.0, 4.0),
                            99,
                        );
                        ui.label(hours_job(
                            total,
                            mono(15.0, true),
                            c.ink,
                            Some((MAX_DAY_HOURS, mono(15.0, true), c.ink)),
                            c.muted,
                        ));
                        bar(ui, ui.available_width(), 8.0, &segments);
                    });
                });
            });
        hline(ui, c.divider);

        if indexes.is_empty() {
            Frame::new()
                .inner_margin(Margin::symmetric(20, 18))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        text(ui, "No time recorded yet.", sans(15.0, W::Regular), c.muted);
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let add = Btn::primary(c, &format!("Add line · {next_hint}"))
                                .show(ui)
                                .on_hover_text(shortcuts::hint(
                                    ui,
                                    "Add line",
                                    &shortcuts::ADD_LINE,
                                ));
                            take_focus(&add, focus, Focus::Day(date));
                            if add.clicked() {
                                actions.push(Action::Add(date));
                            }
                            if let Some(from) = prev {
                                let name = if Month::of(from) == Month::of(date) {
                                    format!("{} {}", weekday_short(from), from.day())
                                } else {
                                    format!(
                                        "{} {} {}",
                                        weekday_short(from),
                                        Month::of(from).short_name(),
                                        from.day()
                                    )
                                };
                                let label = format!("Copy {name}");
                                if Btn::outline(c, &label)
                                    .show(ui)
                                    .on_hover_text(shortcuts::hint(
                                        ui,
                                        &label,
                                        &shortcuts::COPY_DAY,
                                    ))
                                    .clicked()
                                {
                                    actions.push(Action::Copy { from, to: date });
                                }
                            }
                            let day_off = Btn::outline(c, "Day off…")
                                .a11y(format!("Mark {day_name} as a day off"))
                                .show(ui);
                            egui::Popup::menu(&day_off).show(|ui| {
                                for off in DayOff::ALL {
                                    if Btn::base(c, off.label()).show(ui).clicked() {
                                        actions.push(Action::SetDayOff(date, Some(off)));
                                    }
                                }
                            });
                        });
                    });
                });
            return;
        }

        for (k, &i) in indexes.iter().enumerate() {
            if k > 0 {
                hline(ui, c.divider);
            }
            let line_no = k + 1;
            let next_line = indexes.get(k + 1).map(|&n| data.entries[n].id);
            let entry = &mut data.entries[i];
            Frame::new()
                .inner_margin(Margin {
                    left: 20,
                    right: 12,
                    top: 10,
                    bottom: 10,
                })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        dirty |= kind_menu(
                            ui,
                            &mut entry.kind,
                            ("kind", entry.id),
                            &format!("Task type, {day_name} line {line_no}"),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            // Reserve the hours and remove slot so the description is created
                            // first: Tab goes description, hours, remove, then the next button.
                            ui.spacing_mut().item_spacing.x = 6.0;
                            let unit = ui.painter().layout_no_wrap(
                                "h".to_owned(),
                                sans(15.0, W::Regular),
                                c.muted,
                            );
                            let (slot, _) = ui.allocate_exact_size(
                                vec2(76.0 + 6.0 + unit.size().x + 12.0 + 48.0, 44.0),
                                Sense::hover(),
                            );
                            let (desc, chosen) = tickets.desc_field(
                                ui,
                                entry.id,
                                &mut entry.desc,
                                format!("Description, {day_name} line {line_no}"),
                            );
                            dirty |= desc.changed() || chosen;
                            take_focus(&desc, focus, Focus::Desc(entry.id));
                            if k == 0 {
                                take_focus(&desc, focus, Focus::Day(date));
                            }
                            if desc.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                                *focus = Some(Focus::Hours(entry.id));
                            }
                            let slot_ui = UiBuilder::new()
                                .max_rect(slot)
                                .layout(Layout::left_to_right(Align::Center));
                            ui.scope_builder(slot_ui, |ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                ui.scope(|ui| {
                                    if over {
                                        let widgets = &mut ui.visuals_mut().widgets;
                                        widgets.inactive.bg_stroke = Stroke::new(1.0, c.danger);
                                        widgets.hovered.bg_stroke = Stroke::new(1.0, c.danger);
                                    }
                                    // Arrow up/down step the focused field by one hour.
                                    let id = ui.make_persistent_id(("hours", entry.id));
                                    if ui.memory(|m| m.has_focus(id)) {
                                        let delta = ui.input_mut(|i| {
                                            if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                                                1.0
                                            } else if i.consume_key(Modifiers::NONE, Key::ArrowDown)
                                            {
                                                -1.0
                                            } else {
                                                0.0
                                            }
                                        });
                                        if delta != 0.0 {
                                            let stepped = step_hours(&entry.hours, delta);
                                            dirty |= stepped != entry.hours;
                                            entry.hours = stepped;
                                        }
                                    }
                                    let hours = ui.add(
                                        TextEdit::singleline(&mut entry.hours)
                                            .id(id)
                                            .font(mono(15.0, false))
                                            .text_color(c.ink)
                                            .horizontal_align(Align::RIGHT)
                                            .vertical_align(Align::Center)
                                            .margin(Margin {
                                                left: 12,
                                                right: 8,
                                                top: 0,
                                                bottom: 0,
                                            })
                                            .min_size(vec2(76.0, 44.0))
                                            .desired_width(56.0)
                                            .char_limit(5),
                                    );
                                    hours.widget_info(|| {
                                        WidgetInfo::labeled(
                                            WidgetType::TextEdit,
                                            true,
                                            format!("Hours, {day_name} line {line_no}"),
                                        )
                                    });
                                    dirty |= hours.changed();
                                    take_focus(&hours, focus, Focus::Hours(entry.id));
                                    if hours.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter))
                                    {
                                        match next_line {
                                            Some(id) => *focus = Some(Focus::Desc(id)),
                                            None if can_add => actions.push(Action::Add(date)),
                                            None => *focus = next_day.map(Focus::Day),
                                        }
                                    }
                                });
                                ui.spacing_mut().item_spacing.x = 12.0;
                                text(ui, "h", sans(15.0, W::Regular), c.muted);
                                let remove = Btn::base(c, "")
                                    .icon(Icon::Close, 16.0)
                                    .fg(c.muted)
                                    .min(48.0, 44.0)
                                    .a11y(format!("Remove {day_name} line {line_no}"))
                                    .show(ui);
                                if remove.clicked() {
                                    actions.push(Action::Remove(entry.id));
                                }
                            });
                        });
                    });
                });
        }

        // Footer, only when it has something to show.
        if left <= 0.0 && !over {
            return;
        }
        Frame::new()
            .inner_margin(Margin {
                left: 20,
                right: 20,
                top: 10,
                bottom: 14,
            })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if left > 0.0 {
                        let label = format!("Add line · {next_hint}");
                        let mut b = Btn::base(c, &label)
                            .icon(Icon::Plus, 16.0)
                            .fg(c.accent)
                            .font(sans(15.0, W::SemiBold));
                        b.dashed = true;
                        b.pad = 14.0;
                        b.hover_fill = c.accent_tint;
                        let tip = shortcuts::hint(ui, "Add line", &shortcuts::ADD_LINE);
                        if b.show(ui).on_hover_text(tip).clicked() {
                            actions.push(Action::Add(date));
                        }
                    }
                    if over {
                        let msg = format!(
                            "A day cannot exceed 8 h. Remove {} h.",
                            fmt_hours(total - MAX_DAY_HOURS)
                        );
                        text(ui, msg, sans(15.0, W::SemiBold), c.danger);
                    }
                });
            });
    });
    dirty
}

/// "Mon Oct 5" at the start of a day card header.
fn day_heading(ui: &mut Ui, date: NaiveDate) {
    let c = colors(ui);
    ui.horizontal(|ui| {
        ui.set_min_width(140.0);
        ui.spacing_mut().item_spacing.x = 5.0;
        text(ui, weekday_short(date), sans(17.0, W::Bold), c.ink);
        text(
            ui,
            format!("{} {}", Month::of(date).short_name(), date.day()),
            sans(17.0, W::Medium),
            c.muted,
        );
    });
}

/// Weekday marked as a day off: no lines, 0 h toward the target.
fn day_off_card(
    ui: &mut Ui,
    date: NaiveDate,
    label: &str,
    focus: &mut Option<Focus>,
    actions: &mut Vec<Action>,
) {
    let c = colors(ui);
    card(ui, c.border, Margin::symmetric(20, 14), |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            day_heading(ui, date);
            pill(
                ui,
                label,
                c.accent_soft,
                c.accent_strong,
                sans(13.0, W::SemiBold),
                vec2(10.0, 4.0),
                99,
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let workday = Btn::outline(c, "Mark as workday")
                    .a11y(format!("Mark {} as a workday", weekday_long(date)))
                    .show(ui);
                take_focus(&workday, focus, Focus::Day(date));
                if workday.clicked() {
                    actions.push(Action::SetDayOff(date, None));
                }
                text(
                    ui,
                    "Not counted in the target",
                    sans(14.0, W::Regular),
                    c.muted,
                );
            });
        });
    });
}
