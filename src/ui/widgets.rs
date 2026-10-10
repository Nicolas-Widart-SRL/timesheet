//! Small drawing helpers and controls styled like the mockups.

use super::*;

// ---------------------------------------------------------------------------
// Small painted widgets

#[derive(Clone, Copy)]
pub(super) enum Icon {
    ChevronLeft,
    ChevronRight,
    ChevronDown,
    Close,
    Plus,
    Download,
    Check,
    FileCheck,
    Paperclip,
    Folder,
    Monitor,
    Sun,
    Moon,
}

impl Icon {
    /// Paints the 24×24 mockup icon scaled to `size`, centred on `c`.
    pub(super) fn paint(self, p: &Painter, c: Pos2, size: f32, color: Color32, width: f32) {
        let s = size / 24.0;
        let at = |x: f32, y: f32| pos2(c.x + (x - 12.0) * s, c.y + (y - 12.0) * s);
        let stroke = Stroke::new(width, color);
        let line = |pts: &[(f32, f32)]| {
            p.add(Shape::line(
                pts.iter().map(|&(x, y)| at(x, y)).collect(),
                stroke,
            ))
        };
        match self {
            Icon::ChevronLeft => line(&[(15.0, 6.0), (9.0, 12.0), (15.0, 18.0)]),
            Icon::ChevronRight => line(&[(9.0, 6.0), (15.0, 12.0), (9.0, 18.0)]),
            Icon::ChevronDown => line(&[(6.0, 9.0), (12.0, 15.0), (18.0, 9.0)]),
            Icon::Close => {
                line(&[(6.0, 6.0), (18.0, 18.0)]);
                line(&[(18.0, 6.0), (6.0, 18.0)])
            }
            Icon::Plus => {
                line(&[(12.0, 5.0), (12.0, 19.0)]);
                line(&[(5.0, 12.0), (19.0, 12.0)])
            }
            Icon::Download => {
                line(&[(12.0, 4.0), (12.0, 15.0)]);
                line(&[(7.0, 10.0), (12.0, 15.0), (17.0, 10.0)]);
                line(&[(5.0, 20.0), (19.0, 20.0)])
            }
            Icon::Check => line(&[(5.0, 12.5), (10.0, 17.0), (19.0, 7.5)]),
            Icon::FileCheck => {
                line(&[
                    (14.0, 3.0),
                    (5.0, 3.0),
                    (5.0, 21.0),
                    (19.0, 21.0),
                    (19.0, 8.0),
                    (14.0, 3.0),
                    (14.0, 8.0),
                    (19.0, 8.0),
                ]);
                line(&[(9.0, 14.0), (11.0, 16.0), (15.0, 12.0)])
            }
            Icon::Paperclip => {
                // The mockup path is three semicircles joined by straight segments.
                let arc = |pts: &mut Vec<(f32, f32)>, cx: f32, cy: f32, r: f32, from_deg: f32| {
                    for i in 0..=12 {
                        let a = (from_deg + 15.0 * i as f32).to_radians();
                        pts.push((cx + r * a.cos(), cy + r * a.sin()));
                    }
                };
                let mut pts = vec![(21.0, 12.5)];
                arc(&mut pts, 9.0, 17.5, 4.95, 45.0);
                arc(&mut pts, 16.35, 7.85, 3.32, 225.0);
                arc(&mut pts, 9.0, 17.5, 1.7, 45.0);
                pts.push((15.5, 8.6));
                line(&pts)
            }
            Icon::Folder => line(&[
                (3.0, 6.0),
                (9.0, 6.0),
                (11.0, 8.0),
                (21.0, 8.0),
                (21.0, 19.0),
                (3.0, 19.0),
                (3.0, 6.0),
            ]),
            Icon::Monitor => {
                line(&[
                    (3.0, 4.0),
                    (21.0, 4.0),
                    (21.0, 16.0),
                    (3.0, 16.0),
                    (3.0, 4.0),
                ]);
                line(&[(8.0, 20.0), (16.0, 20.0)]);
                line(&[(12.0, 16.0), (12.0, 20.0)])
            }
            Icon::Sun => {
                for i in 0..8 {
                    let a = (45.0 * i as f32).to_radians();
                    let (dx, dy) = (a.cos(), a.sin());
                    line(&[
                        (12.0 + 7.0 * dx, 12.0 + 7.0 * dy),
                        (12.0 + 9.5 * dx, 12.0 + 9.5 * dy),
                    ]);
                }
                p.circle_stroke(at(12.0, 12.0), 4.0 * s, stroke)
            }
            Icon::Moon => {
                // Crescent: the arc of a r=8 circle outside a r=6 circle at (17, 7), then back along the inner one.
                let mut pts = Vec::new();
                for i in 0..=24 {
                    let a = (1.4 + (268.6 - 1.4) * i as f32 / 24.0).to_radians();
                    pts.push((12.0 + 8.0 * a.cos(), 12.0 + 8.0 * a.sin()));
                }
                for i in 0..=12 {
                    let a = (210.0 - 150.0 * i as f32 / 12.0).to_radians();
                    pts.push((17.0 + 6.0 * a.cos(), 7.0 + 6.0 * a.sin()));
                }
                line(&pts)
            }
        };
    }
}

/// Paints the app icon (`assets/icon/icon.svg`) filling the square `rect`: a clock ringed by
/// five weekday arcs, Friday faded. Fixed light-theme colors, like the Dock icon.
pub(super) fn paint_app_mark(p: &Painter, rect: Rect) {
    // SVG units: the tile spans 100..924, the clock is centred on 512.
    let s = rect.width() / 824.0;
    let at = |x: f32, y: f32| pos2(rect.left() + (x - 100.0) * s, rect.top() + (y - 100.0) * s);
    p.rect_filled(rect, CornerRadius::same((185.0 * s) as u8), LIGHT.accent);
    for day in 0..5 {
        // 72° per weekday, minus a 9.17° gap, clockwise from 12 o'clock.
        let start = 72.0 * day as f32 + 4.585;
        let pts = (0..=12)
            .map(|i| {
                let a = (start + 62.83 * i as f32 / 12.0).to_radians();
                at(512.0 + 250.0 * a.sin(), 512.0 - 250.0 * a.cos())
            })
            .collect();
        let color = if day < 4 {
            Color32::WHITE
        } else {
            Color32::WHITE.gamma_multiply(0.3)
        };
        p.add(Shape::line(pts, Stroke::new(72.0 * s, color)));
    }
    let hand = |x: f32, y: f32, width: f32| {
        p.line_segment(
            [at(512.0, 512.0), at(x, y)],
            Stroke::new(width * s, Color32::WHITE),
        );
        p.circle_filled(at(x, y), width * s / 2.0, Color32::WHITE);
    };
    hand(417.0, 457.0, 52.0);
    hand(651.0, 432.0, 40.0);
    p.circle_filled(at(512.0, 512.0), 40.0 * s, Color32::WHITE);
}

#[derive(Clone)]
pub(super) struct Btn<'a> {
    pub(super) text: &'a str,
    pub(super) a11y: Option<String>,
    pub(super) icon: Option<(Icon, f32)>,
    pub(super) font: FontId,
    pub(super) fill: Color32,
    pub(super) hover_fill: Color32,
    pub(super) stroke: Stroke,
    pub(super) dashed: bool,
    pub(super) fg: Color32,
    pub(super) radius: u8,
    pub(super) pad: f32,
    pub(super) min: Vec2,
    pub(super) left: bool,
    pub(super) enabled: bool,
}

impl<'a> Btn<'a> {
    pub(super) fn base(c: &Palette, text: &'a str) -> Self {
        Btn {
            text,
            a11y: None,
            icon: None,
            font: sans(15.0, W::Medium),
            fill: Color32::TRANSPARENT,
            hover_fill: c.chip,
            stroke: Stroke::NONE,
            dashed: false,
            fg: c.ink,
            radius: 10,
            pad: 16.0,
            min: vec2(0.0, 44.0),
            left: false,
            enabled: true,
        }
    }
    pub(super) fn primary(c: &Palette, text: &'a str) -> Self {
        Btn {
            fill: c.accent,
            hover_fill: c.accent_strong,
            fg: c.on_accent,
            font: sans(15.0, W::SemiBold),
            ..Self::base(c, text)
        }
    }
    pub(super) fn outline(c: &Palette, text: &'a str) -> Self {
        Btn {
            fill: c.surface,
            hover_fill: c.bg,
            stroke: Stroke::new(1.0, c.button_border),
            ..Self::base(c, text)
        }
    }
    pub(super) fn icon(mut self, icon: Icon, size: f32) -> Self {
        self.icon = Some((icon, size));
        self
    }
    pub(super) fn a11y(mut self, label: impl Into<String>) -> Self {
        self.a11y = Some(label.into());
        self
    }
    pub(super) fn min(mut self, w: f32, h: f32) -> Self {
        self.min = vec2(w, h);
        self
    }
    pub(super) fn fg(mut self, fg: Color32) -> Self {
        self.fg = fg;
        self
    }
    pub(super) fn font(mut self, font: FontId) -> Self {
        self.font = font;
        self
    }

    pub(super) fn show(self, ui: &mut Ui) -> Response {
        let c = colors(ui);
        let galley = ui
            .painter()
            .layout_no_wrap(self.text.to_owned(), self.font.clone(), self.fg);
        let icon_w = match self.icon {
            Some((_, s)) if self.text.is_empty() => s,
            Some((_, s)) => s + 8.0,
            None => 0.0,
        };
        let content_w = icon_w + galley.size().x;
        let size = vec2((content_w + 2.0 * self.pad).max(self.min.x), self.min.y);
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(size, sense);
        let label = self.a11y.clone().unwrap_or_else(|| self.text.to_owned());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, self.enabled, &label));

        if ui.is_rect_visible(rect) {
            let p = ui.painter();
            let alpha = if self.enabled { 1.0 } else { 0.45 };
            let fill = if self.enabled && response.hovered() {
                self.hover_fill
            } else {
                self.fill
            };
            p.rect_filled(rect, self.radius, fill.gamma_multiply(alpha));
            if self.dashed {
                dashed_rect(p, rect.shrink(0.5), Stroke::new(1.0, c.hover_border));
            } else if self.stroke.width > 0.0 {
                p.rect_stroke(rect, self.radius, self.stroke, StrokeKind::Inside);
            }
            focus_ring(ui, &response, rect, self.radius);
            let mut x = if self.left {
                rect.left() + self.pad
            } else {
                rect.center().x - content_w / 2.0
            };
            let fg = self.fg.gamma_multiply(alpha);
            if let Some((icon, s)) = self.icon {
                icon.paint(p, pos2(x + s / 2.0, rect.center().y), s, fg, 2.0);
                x += icon_w;
            }
            p.galley(pos2(x, rect.center().y - galley.size().y / 2.0), galley, fg);
        }
        if self.enabled {
            response.on_hover_cursor(CursorIcon::PointingHand)
        } else {
            response
        }
    }
}

pub(super) fn focus_ring(ui: &Ui, response: &Response, rect: Rect, radius: u8) {
    let c = colors(ui);
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(2.0),
            radius + 2,
            Stroke::new(2.0, c.accent),
            StrokeKind::Outside,
        );
    }
}

pub(super) fn dashed_rect(p: &Painter, r: Rect, stroke: Stroke) {
    let pts = [
        r.left_top(),
        r.right_top(),
        r.right_bottom(),
        r.left_bottom(),
        r.left_top(),
    ];
    for pair in pts.windows(2) {
        p.extend(Shape::dashed_line(pair, stroke, 4.0, 3.0));
    }
}

pub(super) fn text(ui: &mut Ui, s: impl Into<String>, font: FontId, color: Color32) -> Response {
    ui.label(egui::RichText::new(s).font(font).color(color))
}

/// Hours as mono figures with "/" and "h" in muted sans, tightly spaced: "6 / 8 h", "176 h".
/// The target figure has its own font and color; the units take the size of the figure before them.
pub(super) fn hours_job(
    total: f64,
    font: FontId,
    color: Color32,
    target: Option<(f64, FontId, Color32)>,
    muted: Color32,
) -> LayoutJob {
    let gap = |f: &FontId| (f.size * 0.2).round();
    let unit = |f: &FontId| egui::TextFormat::simple(sans(f.size, W::Regular), muted);
    let mut job = LayoutJob::default();
    job.append(
        &fmt_hours(total),
        0.0,
        egui::TextFormat::simple(font.clone(), color),
    );
    let mut last = font;
    if let Some((target, font, color)) = target {
        job.append("/", gap(&font), unit(&font));
        job.append(
            &fmt_hours(target),
            gap(&font),
            egui::TextFormat::simple(font.clone(), color),
        );
        last = font;
    }
    job.append("h", gap(&last), unit(&last));
    job
}

pub(super) fn pill(
    ui: &mut Ui,
    s: &str,
    bg: Color32,
    fg: Color32,
    font: FontId,
    pad: Vec2,
    radius: u8,
) -> Response {
    let galley = ui.painter().layout_no_wrap(s.to_owned(), font, fg);
    let (rect, response) = ui.allocate_exact_size(galley.size() + 2.0 * pad, Sense::hover());
    ui.painter().rect_filled(rect, radius, bg);
    ui.painter().galley(rect.min + pad, galley, fg);
    response
}

pub(super) fn swatch(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
    ui.painter().rect_filled(rect, 3, color);
}

/// Horizontal stacked bar; `segments` are fractions of the full width.
pub(super) fn paint_bar(p: &Painter, rect: Rect, track: Color32, segments: &[(f32, Color32)]) {
    let r = (rect.height() / 2.0).round() as u8;
    p.rect_filled(rect, r, track);
    let mut x = rect.left();
    let visible: Vec<_> = segments.iter().filter(|(f, _)| *f > 0.0).collect();
    for (i, &&(fraction, color)) in visible.iter().enumerate() {
        let w = (fraction.min(1.0) * rect.width()).min(rect.right() - x);
        if w <= 0.0 {
            break;
        }
        let seg = Rect::from_min_size(pos2(x, rect.top()), vec2(w, rect.height()));
        let left = if i == 0 { r } else { 0 };
        let right = if seg.right() >= rect.right() - 0.5 {
            r
        } else {
            0
        };
        p.rect_filled(
            seg,
            CornerRadius {
                nw: left,
                sw: left,
                ne: right,
                se: right,
            },
            color,
        );
        x += w;
    }
}

pub(super) fn bar(ui: &mut Ui, width: f32, height: f32, segments: &[(f32, Color32)]) {
    let c = colors(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(width.max(0.0), height), Sense::hover());
    paint_bar(ui.painter(), rect, c.divider, segments);
}

pub(super) fn card<R>(
    ui: &mut Ui,
    border: Color32,
    margin: impl Into<Margin>,
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    let c = colors(ui);
    Frame::new()
        .fill(c.surface)
        .stroke(Stroke::new(1.0, border))
        .corner_radius(14)
        .inner_margin(margin)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// Small uppercase column heading.
pub(super) fn caption(ui: &mut Ui, s: &str) {
    let c = colors(ui);
    ui.label(
        egui::RichText::new(s.to_uppercase())
            .font(sans(12.0, W::SemiBold))
            .color(c.muted)
            .extra_letter_spacing(0.72),
    );
}

/// 22 px checkbox with a semibold label, in a 44 px tall row. Returns a changed response when toggled.
pub(super) fn checkbox(
    ui: &mut Ui,
    checked: &mut bool,
    label: &str,
    accent: Color32,
    enabled: bool,
) -> Response {
    let c = colors(ui);
    let fg = if enabled { c.ink } else { c.disabled };
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), sans(15.0, W::SemiBold), fg);
    let size = vec2(22.0 + 10.0 + galley.size().x, 44.0);
    let (rect, mut response) = ui.allocate_exact_size(
        size,
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    if response.clicked() {
        *checked = !*checked;
        response.mark_changed();
    }
    response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, enabled, *checked, label));
    let p = ui.painter();
    let box_rect =
        Rect::from_center_size(pos2(rect.left() + 11.0, rect.center().y), vec2(22.0, 22.0));
    let alpha = if enabled { 1.0 } else { 0.45 };
    if *checked {
        p.rect_filled(box_rect, 5, accent.gamma_multiply(alpha));
        Icon::Check.paint(p, box_rect.center(), 18.0, c.on_accent, 2.5);
    } else {
        let border = if enabled && response.hovered() {
            accent
        } else {
            c.muted
        };
        p.rect_filled(box_rect, 5, c.surface);
        p.rect_stroke(
            box_rect,
            5,
            Stroke::new(1.5, border.gamma_multiply(alpha)),
            StrokeKind::Inside,
        );
    }
    p.galley(
        pos2(
            box_rect.right() + 10.0,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        fg,
    );
    focus_ring(ui, &response, rect, 8);
    response.on_hover_cursor(if enabled {
        CursorIcon::PointingHand
    } else {
        CursorIcon::NotAllowed
    })
}

/// Date picker button styled like the mockup's date input. Returns true when the date changed.
pub(super) fn date_field(ui: &mut Ui, date: &mut NaiveDate, id_salt: &str, a11y: &str) -> bool {
    let c = colors(ui);
    let mut picked =
        jiff::civil::Date::new(date.year() as i16, date.month() as i8, date.day() as i8)
            .unwrap_or_default();
    let response = ui
        .scope(|ui| {
            ui.spacing_mut().interact_size.y = 38.0;
            ui.spacing_mut().button_padding = vec2(10.0, 0.0);
            let w = &mut ui.visuals_mut().widgets;
            for state in [&mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
                state.weak_bg_fill = c.surface;
                state.corner_radius = CornerRadius::same(8);
            }
            ui.add(
                egui_extras::DatePickerButton::new(&mut picked)
                    .id_salt(id_salt)
                    .show_icon(false)
                    .calendar_week(false)
                    .highlight_weekends(false),
            )
        })
        .inner;
    response
        .widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, format!("{a11y}: {date}")));
    let new = NaiveDate::from_ymd_opt(
        picked.year().into(),
        picked.month() as u32,
        picked.day() as u32,
    );
    match new {
        Some(d) if response.changed() && d != *date => {
            *date = d;
            true
        }
        _ => false,
    }
}

/// Full-width dropdown styled like the 44px settings text fields, with a chevron.
pub(super) fn select_field<T: Copy + PartialEq>(
    ui: &mut Ui,
    id_salt: &str,
    a11y: &str,
    value: &mut T,
    options: impl IntoIterator<Item = T>,
    label: impl Fn(T) -> &'static str,
) {
    const HEIGHT: f32 = 44.0;
    const CHEVRON: f32 = 16.0;
    let c = colors(ui);
    let font = sans(15.0, W::Regular);
    let row = ui.fonts_mut(|f| f.row_height(&font)).max(CHEVRON);
    ui.scope(|ui| {
        let s = ui.spacing_mut();
        s.interact_size.y = HEIGHT;
        // ComboBox top-aligns its content, so vertical padding centres it.
        s.button_padding = vec2(12.0, (HEIGHT - row) / 2.0);
        s.icon_width = CHEVRON;
        let focus = ui.visuals().selection.stroke;
        let w = &mut ui.visuals_mut().widgets;
        for state in [&mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
            state.weak_bg_fill = c.surface;
        }
        w.open.bg_stroke = focus;
        let response = egui::ComboBox::from_id_salt(id_salt)
            .selected_text(egui::RichText::new(label(*value)).font(font).color(c.ink))
            .width(ui.available_width())
            .icon(move |ui, rect, _, _| {
                Icon::ChevronDown.paint(ui.painter(), rect.center(), CHEVRON, c.muted, 2.0);
            })
            .show_ui(ui, |ui| {
                for option in options {
                    ui.selectable_value(value, option, label(option));
                }
            })
            .response;
        let radius = ui.visuals().widgets.inactive.corner_radius.nw;
        focus_ring(ui, &response, response.rect, radius);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::ComboBox, true, a11y));
    });
}

/// 132 px outline button with the task type's swatch and name; a click opens a menu of all task
/// types. Returns true when the task type changed.
pub(super) fn kind_menu(
    ui: &mut Ui,
    value: &mut Kind,
    id_salt: impl egui::AsIdSalt,
    a11y: &str,
) -> bool {
    let c = colors(ui);
    let font = sans(14.0, W::Medium);
    let (_, rect) = ui.allocate_space(vec2(132.0, 44.0));
    let response = ui.interact(rect, ui.id().with(id_salt), Sense::click());
    let popup_id = egui::Popup::default_response_id(&response);
    let mut open = egui::Popup::is_id_open(ui.ctx(), popup_id);
    let mut changed = false;
    // F / B / S pick a task type on the focused button or in its open menu, which then closes.
    if response.has_focus() || open {
        let typed = ui.input_mut(|i| {
            Kind::ALL
                .into_iter()
                .find(|&k| i.consume_key(Modifiers::NONE, super::shortcuts::kind_key(k)))
        });
        if let Some(kind) = typed {
            changed = *value != kind;
            *value = kind;
            if open {
                egui::Popup::close_id(ui.ctx(), popup_id);
                response.request_focus();
                open = false;
            }
        }
    }
    response.widget_info(|| {
        WidgetInfo::labeled(
            WidgetType::ComboBox,
            true,
            format!("{a11y}: {}", value.label()),
        )
    });
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        let fill = if response.hovered() || open {
            c.bg
        } else {
            c.surface
        };
        p.rect_filled(rect, 10, fill);
        p.rect_stroke(
            rect,
            10,
            Stroke::new(1.0, c.button_border),
            StrokeKind::Inside,
        );
        focus_ring(ui, &response, rect, 10);
        paint_kind_label(p, rect, 12.0, *value, font.clone(), c);
        Icon::ChevronDown.paint(
            p,
            pos2(rect.right() - 18.0, rect.center().y),
            16.0,
            c.muted,
            2.0,
        );
    }
    egui::Popup::menu(&response).width(180.0).show(|ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        for kind in Kind::ALL {
            let selected = *value == kind;
            let (rect, item) =
                ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::click());
            item.widget_info(|| {
                WidgetInfo::selected(WidgetType::RadioButton, true, selected, kind.label())
            });
            let p = ui.painter();
            if item.hovered() || item.has_focus() {
                p.rect_filled(rect, 8, c.chip);
            }
            paint_kind_label(p, rect, 10.0, kind, font.clone(), c);
            if selected {
                Icon::Check.paint(
                    p,
                    pos2(rect.right() - 18.0, rect.center().y),
                    16.0,
                    c.ink,
                    2.0,
                );
            }
            if item.on_hover_cursor(CursorIcon::PointingHand).clicked() && !selected {
                *value = kind;
                changed = true;
            }
        }
    });
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text("Task type (F / B / S)");
    changed
}

/// Task-type swatch followed by its name, `pad` in from the left of `rect`.
fn paint_kind_label(p: &Painter, rect: Rect, pad: f32, kind: Kind, font: FontId, c: &Palette) {
    let swatch = Rect::from_min_size(
        pos2(rect.left() + pad, rect.center().y - 5.0),
        vec2(10.0, 10.0),
    );
    p.rect_filled(swatch, 3, c.kind(kind));
    let galley = p.layout_no_wrap(kind.label().to_owned(), font, c.ink);
    p.galley(
        pos2(
            swatch.right() + 8.0,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        c.ink,
    );
}

/// Full-width radio group of 36 px segments on a `chip` track; the selected one is raised.
/// Returns true when the selection changed.
pub(super) fn segmented<T: Copy + PartialEq>(
    ui: &mut Ui,
    value: &mut T,
    options: &[(T, Icon, &str)],
) -> bool {
    const PAD: f32 = 4.0;
    let c = colors(ui);
    let (track, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), 36.0 + 2.0 * PAD), Sense::hover());
    ui.painter().rect_filled(track, 10, c.chip);
    let inner = track.shrink(PAD);
    let w = (inner.width() - PAD * (options.len() as f32 - 1.0)) / options.len() as f32;
    let mut changed = false;
    for (i, &(option, icon, label)) in options.iter().enumerate() {
        let rect = Rect::from_min_size(
            pos2(inner.left() + i as f32 * (w + PAD), inner.top()),
            vec2(w, inner.height()),
        );
        let response = ui.interact(rect, ui.id().with(("segment", i)), Sense::click());
        if response.clicked() && *value != option {
            *value = option;
            changed = true;
        }
        let selected = *value == option;
        response
            .widget_info(|| WidgetInfo::selected(WidgetType::RadioButton, true, selected, label));
        let p = ui.painter();
        if selected {
            p.rect_filled(rect, 8, c.surface);
            p.rect_stroke(
                rect,
                8,
                Stroke::new(1.0, c.button_border),
                StrokeKind::Inside,
            );
        } else if response.hovered() {
            p.rect_filled(rect, 8, c.border);
        }
        focus_ring(ui, &response, rect, 8);
        let fg = if selected { c.ink } else { c.ink2 };
        let galley = p.layout_no_wrap(label.to_owned(), sans(14.0, W::Medium), fg);
        let x = rect.center().x - (16.0 + 6.0 + galley.size().x) / 2.0;
        icon.paint(p, pos2(x + 8.0, rect.center().y), 16.0, fg, 2.0);
        p.galley(
            pos2(x + 22.0, rect.center().y - galley.size().y / 2.0),
            galley,
            fg,
        );
        response.on_hover_cursor(CursorIcon::PointingHand);
    }
    changed
}

/// CSS-like `flex-wrap` row: each item gets at least its `basis` width and lines share leftover width by `grow`.
pub(super) fn flex_wrap(
    ui: &mut Ui,
    items: &[(f32, f32)],
    gap: Vec2,
    mut add: impl FnMut(&mut Ui, usize),
) {
    let width = ui.available_width();
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = gap.y;
        let mut start = 0;
        while start < items.len() {
            let mut end = start + 1;
            let mut used = items[start].0;
            while end < items.len() && used + gap.x + items[end].0 <= width {
                used += gap.x + items[end].0;
                end += 1;
            }
            let line = &items[start..end];
            let grow: f32 = line.iter().map(|i| i.1).sum();
            let extra = (width - used).max(0.0);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap.x;
                for (k, &(basis, g)) in line.iter().enumerate() {
                    let w = (basis + extra * g / grow).min(width);
                    ui.allocate_ui_with_layout(vec2(w, 0.0), Layout::top_down(Align::Min), |ui| {
                        ui.set_width(w);
                        add(ui, start + k);
                    });
                }
            });
            start = end;
        }
    });
}

/// Green success or red error line under an action.
pub(super) fn note_ui(ui: &mut Ui, note: &Result<String, String>) {
    let c = colors(ui);
    let (msg, weight, color) = match note {
        Ok(msg) => (msg, W::Regular, c.ok_fg),
        Err(msg) => (msg, W::SemiBold, c.danger),
    };
    ui.add(
        egui::Label::new(
            egui::RichText::new(msg)
                .font(sans(13.0, weight))
                .color(color),
        )
        .wrap(),
    );
}

pub(super) fn hline(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter()
        .hline(rect.x_range(), rect.center().y, Stroke::new(1.0, color));
}

/// Places `add` in a horizontally centred column of at most `max_w` (including `pad` on both sides).
pub(super) fn centered<R>(ui: &mut Ui, max_w: f32, pad: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let avail = ui.available_rect_before_wrap();
    let w = (avail.width().min(max_w) - 2.0 * pad).max(100.0);
    let left = avail.left() + (avail.width() - w) / 2.0;
    let rect = Rect::from_min_size(pos2(left, avail.top()), vec2(w, avail.height().max(0.0)));
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::top_down(Align::Min)),
        |ui| {
            ui.set_width(w);
            add(ui)
        },
    )
    .inner
}
