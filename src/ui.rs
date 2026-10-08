//! Desktop UI replicating the mockups in `mockups/`: monthly entry and past months (sent & approval tracking).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{Datelike, NaiveDate};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, CursorIcon, FontData, FontDefinitions, FontFamily, FontId, Frame,
    Id, Layout, Margin, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, TextEdit, Ui, UiBuilder, Vec2,
    WidgetInfo, WidgetType, pos2, text::LayoutJob, vec2,
};

use crate::fonts;
use crate::model::{
    Approval, ApprovalStatus, Data, Kind, MAX_DAY_HOURS, Month, Settings, Store, day_range, fmt_hours, weekday_long,
    weekday_short,
};
use crate::pdf;

// Palette from the mockup.
const BG: Color32 = Color32::from_rgb(0xF4, 0xF5, 0xF7);
const WHITE: Color32 = Color32::WHITE;
const INK: Color32 = Color32::from_rgb(0x15, 0x17, 0x1C);
const INK2: Color32 = Color32::from_rgb(0x3F, 0x44, 0x52);
const MUTED: Color32 = Color32::from_rgb(0x5B, 0x61, 0x70);
const BORDER: Color32 = Color32::from_rgb(0xE2, 0xE4, 0xE9);
const INPUT_BORDER: Color32 = Color32::from_rgb(0xD5, 0xD8, 0xDF);
const HOVER_BORDER: Color32 = Color32::from_rgb(0xB9, 0xBE, 0xC9);
const DIVIDER: Color32 = Color32::from_rgb(0xEE, 0xF0, 0xF3);
const CHIP: Color32 = Color32::from_rgb(0xF0, 0xF1, 0xF4);
const ACCENT: Color32 = Color32::from_rgb(0x24, 0x46, 0xD8);
const ACCENT_DARK: Color32 = Color32::from_rgb(0x16, 0x30, 0x9E);
const ACCENT_SOFT: Color32 = Color32::from_rgb(0xE8, 0xEC, 0xFC);
const ACCENT_TINT: Color32 = Color32::from_rgb(0xF5, 0xF7, 0xFF);
const DANGER: Color32 = Color32::from_rgb(0xB4, 0x23, 0x18);
const DANGER_SOFT: Color32 = Color32::from_rgb(0xFD, 0xEC, 0xEA);
const OK: Color32 = Color32::from_rgb(0x0F, 0x7B, 0x6C);
const OK_FG: Color32 = Color32::from_rgb(0x0A, 0x52, 0x48);
const OK_SOFT: Color32 = Color32::from_rgb(0xDD, 0xF3, 0xEF);
const OK_BORDER: Color32 = Color32::from_rgb(0xBF, 0xE3, 0xDC);
const OK_TINT: Color32 = Color32::from_rgb(0xF1, 0xFA, 0xF8);
const WARN: Color32 = Color32::from_rgb(0x8A, 0x3C, 0x08);
const WARN_SOFT: Color32 = Color32::from_rgb(0xFF, 0xF1, 0xE6);
const WARN_BORDER: Color32 = Color32::from_rgb(0xE9, 0xB4, 0x8C);
const DISABLED: Color32 = Color32::from_rgb(0x8A, 0x8F, 0x9C);

fn kind_color(kind: Kind) -> Color32 {
    let (r, g, b) = pdf::kind_rgb(kind);
    Color32::from_rgb(r, g, b)
}

/// Fill of a selected task-type toggle (backend is darkened for text contrast).
fn kind_toggle_fill(kind: Kind) -> Color32 {
    match kind {
        Kind::Backend => Color32::from_rgb(0xA4, 0x4A, 0x0C),
        k => kind_color(k),
    }
}

#[derive(Clone, Copy)]
enum W {
    Regular,
    Medium,
    SemiBold,
    Bold,
}

fn sans(size: f32, weight: W) -> FontId {
    let family = match weight {
        W::Regular => FontFamily::Proportional,
        W::Medium => FontFamily::Name("medium".into()),
        W::SemiBold => FontFamily::Name("semibold".into()),
        W::Bold => FontFamily::Name("bold".into()),
    };
    FontId::new(size, family)
}

fn mono(size: f32, semibold: bool) -> FontId {
    if semibold { FontId::new(size, FontFamily::Name("mono-semibold".into())) } else { FontId::monospace(size) }
}

fn setup_fonts(ctx: &egui::Context) {
    let mut defs = FontDefinitions::default();
    let faces = [
        ("sans-regular", fonts::SANS_REGULAR),
        ("sans-medium", fonts::SANS_MEDIUM),
        ("sans-semibold", fonts::SANS_SEMIBOLD),
        ("sans-bold", fonts::SANS_BOLD),
        ("mono-regular", fonts::MONO_REGULAR),
        ("mono-semibold", fonts::MONO_SEMIBOLD),
    ];
    for (name, bytes) in faces {
        defs.font_data.insert(name.into(), Arc::new(FontData::from_static(bytes)));
    }
    let fallback = defs.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    defs.families.entry(FontFamily::Proportional).or_default().insert(0, "sans-regular".into());
    defs.families.entry(FontFamily::Monospace).or_default().insert(0, "mono-regular".into());
    for (family, face) in [
        ("medium", "sans-medium"),
        ("semibold", "sans-semibold"),
        ("bold", "sans-bold"),
        ("mono-semibold", "mono-semibold"),
    ] {
        let mut list = vec![face.to_owned()];
        list.extend(fallback.iter().cloned());
        defs.families.insert(FontFamily::Name(family.into()), list);
    }
    ctx.set_fonts(defs);
}

fn setup_style(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Light);
    ctx.all_styles_mut(|style| {
        style.text_styles.insert(egui::TextStyle::Body, sans(15.0, W::Regular));
        style.text_styles.insert(egui::TextStyle::Button, sans(15.0, W::Regular));
        style.text_styles.insert(egui::TextStyle::Heading, sans(30.0, W::Bold));
        style.text_styles.insert(egui::TextStyle::Small, sans(13.0, W::Regular));
        style.spacing.item_spacing = vec2(8.0, 8.0);
        let v = &mut style.visuals;
        v.panel_fill = BG;
        v.window_fill = WHITE;
        v.extreme_bg_color = WHITE;
        v.override_text_color = None;
        v.selection.bg_fill = Color32::from_rgb(0xC9, 0xD3, 0xFA);
        v.selection.stroke = Stroke::new(2.0, ACCENT);
        v.text_cursor.stroke = Stroke::new(2.0, ACCENT);
        v.hyperlink_color = ACCENT;
        let radius = CornerRadius::same(10);
        for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
            w.corner_radius = radius;
            w.expansion = 0.0;
            w.fg_stroke.color = INK;
        }
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, INPUT_BORDER);
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, HOVER_BORDER);
        v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    });
}

// ---------------------------------------------------------------------------
// Small painted widgets

#[derive(Clone, Copy)]
enum Icon {
    ChevronLeft,
    ChevronRight,
    Close,
    Plus,
    Download,
    Clock,
    Check,
    FileCheck,
    Paperclip,
}

impl Icon {
    /// Paints the 24×24 mockup icon scaled to `size`, centred on `c`.
    fn paint(self, p: &Painter, c: Pos2, size: f32, color: Color32, width: f32) {
        let s = size / 24.0;
        let at = |x: f32, y: f32| pos2(c.x + (x - 12.0) * s, c.y + (y - 12.0) * s);
        let stroke = Stroke::new(width, color);
        let line = |pts: &[(f32, f32)]| p.add(Shape::line(pts.iter().map(|&(x, y)| at(x, y)).collect(), stroke));
        match self {
            Icon::ChevronLeft => line(&[(15.0, 6.0), (9.0, 12.0), (15.0, 18.0)]),
            Icon::ChevronRight => line(&[(9.0, 6.0), (15.0, 12.0), (9.0, 18.0)]),
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
            Icon::Clock => {
                p.circle_stroke(at(12.0, 12.0), 9.0 * s, stroke);
                line(&[(12.0, 7.0), (12.0, 12.0), (15.0, 14.0)])
            }
            Icon::Check => line(&[(5.0, 12.5), (10.0, 17.0), (19.0, 7.5)]),
            Icon::FileCheck => {
                line(&[(14.0, 3.0), (5.0, 3.0), (5.0, 21.0), (19.0, 21.0), (19.0, 8.0), (14.0, 3.0), (14.0, 8.0), (19.0, 8.0)]);
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
        };
    }
}

#[derive(Clone)]
struct Btn<'a> {
    text: &'a str,
    a11y: Option<String>,
    icon: Option<(Icon, f32)>,
    font: FontId,
    fill: Color32,
    hover_fill: Color32,
    stroke: Stroke,
    dashed: bool,
    fg: Color32,
    radius: u8,
    pad: f32,
    min: Vec2,
    left: bool,
    enabled: bool,
}

impl<'a> Btn<'a> {
    fn base(text: &'a str) -> Self {
        Btn {
            text,
            a11y: None,
            icon: None,
            font: sans(15.0, W::Medium),
            fill: Color32::TRANSPARENT,
            hover_fill: CHIP,
            stroke: Stroke::NONE,
            dashed: false,
            fg: INK,
            radius: 10,
            pad: 16.0,
            min: vec2(0.0, 44.0),
            left: false,
            enabled: true,
        }
    }
    fn primary(text: &'a str) -> Self {
        Btn { fill: ACCENT, hover_fill: ACCENT_DARK, fg: WHITE, font: sans(15.0, W::SemiBold), ..Self::base(text) }
    }
    fn outline(text: &'a str) -> Self {
        Btn { fill: WHITE, hover_fill: BG, stroke: Stroke::new(1.0, INPUT_BORDER), ..Self::base(text) }
    }
    fn icon(mut self, icon: Icon, size: f32) -> Self {
        self.icon = Some((icon, size));
        self
    }
    fn a11y(mut self, label: impl Into<String>) -> Self {
        self.a11y = Some(label.into());
        self
    }
    fn min(mut self, w: f32, h: f32) -> Self {
        self.min = vec2(w, h);
        self
    }
    fn fg(mut self, fg: Color32) -> Self {
        self.fg = fg;
        self
    }
    fn font(mut self, font: FontId) -> Self {
        self.font = font;
        self
    }

    fn show(self, ui: &mut Ui) -> Response {
        let galley = ui.painter().layout_no_wrap(self.text.to_owned(), self.font.clone(), self.fg);
        let icon_w = match self.icon {
            Some((_, s)) if self.text.is_empty() => s,
            Some((_, s)) => s + 8.0,
            None => 0.0,
        };
        let content_w = icon_w + galley.size().x;
        let size = vec2((content_w + 2.0 * self.pad).max(self.min.x), self.min.y);
        let sense = if self.enabled { Sense::click() } else { Sense::hover() };
        let (rect, response) = ui.allocate_exact_size(size, sense);
        let label = self.a11y.clone().unwrap_or_else(|| self.text.to_owned());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, self.enabled, &label));

        if ui.is_rect_visible(rect) {
            let p = ui.painter();
            let alpha = if self.enabled { 1.0 } else { 0.45 };
            let fill = if self.enabled && response.hovered() { self.hover_fill } else { self.fill };
            p.rect_filled(rect, self.radius, fill.gamma_multiply(alpha));
            if self.dashed {
                dashed_rect(p, rect.shrink(0.5), Stroke::new(1.0, HOVER_BORDER));
            } else if self.stroke.width > 0.0 {
                p.rect_stroke(rect, self.radius, self.stroke, StrokeKind::Inside);
            }
            focus_ring(ui, &response, rect, self.radius);
            let mut x = if self.left { rect.left() + self.pad } else { rect.center().x - content_w / 2.0 };
            let fg = self.fg.gamma_multiply(alpha);
            if let Some((icon, s)) = self.icon {
                icon.paint(p, pos2(x + s / 2.0, rect.center().y), s, fg, 2.0);
                x += icon_w;
            }
            p.galley(pos2(x, rect.center().y - galley.size().y / 2.0), galley, fg);
        }
        if self.enabled { response.on_hover_cursor(CursorIcon::PointingHand) } else { response }
    }
}

fn focus_ring(ui: &Ui, response: &Response, rect: Rect, radius: u8) {
    if response.has_focus() {
        ui.painter().rect_stroke(rect.expand(2.0), radius + 2, Stroke::new(2.0, ACCENT), StrokeKind::Outside);
    }
}

fn dashed_rect(p: &Painter, r: Rect, stroke: Stroke) {
    let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    for pair in pts.windows(2) {
        p.extend(Shape::dashed_line(pair, stroke, 4.0, 3.0));
    }
}

fn text(ui: &mut Ui, s: impl Into<String>, font: FontId, color: Color32) -> Response {
    ui.label(egui::RichText::new(s).font(font).color(color))
}

fn pill(ui: &mut Ui, s: &str, bg: Color32, fg: Color32, font: FontId, pad: Vec2, radius: u8) -> Response {
    let galley = ui.painter().layout_no_wrap(s.to_owned(), font, fg);
    let (rect, response) = ui.allocate_exact_size(galley.size() + 2.0 * pad, Sense::hover());
    ui.painter().rect_filled(rect, radius, bg);
    ui.painter().galley(rect.min + pad, galley, fg);
    response
}

fn swatch(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
    ui.painter().rect_filled(rect, 3, color);
}

/// Horizontal stacked bar; `segments` are fractions of the full width.
fn paint_bar(p: &Painter, rect: Rect, track: Color32, segments: &[(f32, Color32)]) {
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
        let right = if seg.right() >= rect.right() - 0.5 { r } else { 0 };
        p.rect_filled(seg, CornerRadius { nw: left, sw: left, ne: right, se: right }, color);
        x += w;
    }
}

fn bar(ui: &mut Ui, width: f32, height: f32, segments: &[(f32, Color32)]) {
    let (rect, _) = ui.allocate_exact_size(vec2(width.max(0.0), height), Sense::hover());
    paint_bar(ui.painter(), rect, DIVIDER, segments);
}

fn card<R>(ui: &mut Ui, border: Color32, margin: impl Into<Margin>, add: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::new()
        .fill(WHITE)
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
fn caption(ui: &mut Ui, s: &str) {
    ui.label(egui::RichText::new(s.to_uppercase()).font(sans(12.0, W::SemiBold)).color(MUTED).extra_letter_spacing(0.72));
}

/// 22 px checkbox with a semibold label, in a 44 px tall row. Returns a changed response when toggled.
fn checkbox(ui: &mut Ui, checked: &mut bool, label: &str, accent: Color32, enabled: bool) -> Response {
    let fg = if enabled { INK } else { DISABLED };
    let galley = ui.painter().layout_no_wrap(label.to_owned(), sans(15.0, W::SemiBold), fg);
    let size = vec2(22.0 + 10.0 + galley.size().x, 44.0);
    let (rect, mut response) = ui.allocate_exact_size(size, if enabled { Sense::click() } else { Sense::hover() });
    if response.clicked() {
        *checked = !*checked;
        response.mark_changed();
    }
    response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, enabled, *checked, label));
    let p = ui.painter();
    let box_rect = Rect::from_center_size(pos2(rect.left() + 11.0, rect.center().y), vec2(22.0, 22.0));
    let alpha = if enabled { 1.0 } else { 0.45 };
    if *checked {
        p.rect_filled(box_rect, 5, accent.gamma_multiply(alpha));
        Icon::Check.paint(p, box_rect.center(), 18.0, WHITE, 2.5);
    } else {
        let border = if enabled && response.hovered() { accent } else { MUTED };
        p.rect_filled(box_rect, 5, WHITE);
        p.rect_stroke(box_rect, 5, Stroke::new(1.5, border.gamma_multiply(alpha)), StrokeKind::Inside);
    }
    p.galley(pos2(box_rect.right() + 10.0, rect.center().y - galley.size().y / 2.0), galley, fg);
    focus_ring(ui, &response, rect, 8);
    response.on_hover_cursor(if enabled { CursorIcon::PointingHand } else { CursorIcon::NotAllowed })
}

/// Date picker button styled like the mockup's date input. Returns true when the date changed.
fn date_field(ui: &mut Ui, date: &mut NaiveDate, id_salt: &str, a11y: &str) -> bool {
    let mut picked = jiff::civil::Date::new(date.year() as i16, date.month() as i8, date.day() as i8).unwrap_or_default();
    let response = ui
        .scope(|ui| {
            ui.spacing_mut().interact_size.y = 38.0;
            ui.spacing_mut().button_padding = vec2(10.0, 0.0);
            let w = &mut ui.visuals_mut().widgets;
            for state in [&mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
                state.weak_bg_fill = WHITE;
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
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, format!("{a11y}: {date}")));
    let new = NaiveDate::from_ymd_opt(picked.year().into(), picked.month() as u32, picked.day() as u32);
    match new {
        Some(d) if response.changed() && d != *date => {
            *date = d;
            true
        }
        _ => false,
    }
}

/// CSS-like `flex-wrap` row: each item gets at least its `basis` width and lines share leftover width by `grow`.
fn flex_wrap(ui: &mut Ui, items: &[(f32, f32)], gap: Vec2, mut add: impl FnMut(&mut Ui, usize)) {
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

/// Opens `path` with the system's default app.
fn open_path(path: &Path) -> std::io::Result<()> {
    if !path.exists() {
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "file is missing"));
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
fn progress_status(data: &Data, month: Month) -> (&'static str, Color32, Color32) {
    if data.has_overfull_day(month) {
        ("Needs fixes", DANGER_SOFT, DANGER)
    } else if month.weekdays().iter().all(|&d| data.day_total(d) >= MAX_DAY_HOURS) {
        ("Complete", OK_SOFT, OK_FG)
    } else if data.month_total(month) > 0.0 {
        ("In progress", CHIP, INK2)
    } else {
        ("Empty", CHIP, INK2)
    }
}

fn hline(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().hline(rect.x_range(), rect.center().y, Stroke::new(1.0, color));
}

/// Places `add` in a horizontally centred column of at most `max_w` (including `pad` on both sides).
fn centered<R>(ui: &mut Ui, max_w: f32, pad: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let avail = ui.available_rect_before_wrap();
    let w = (avail.width().min(max_w) - 2.0 * pad).max(100.0);
    let left = avail.left() + (avail.width() - w) / 2.0;
    let rect = Rect::from_min_size(pos2(left, avail.top()), vec2(w, avail.height().max(0.0)));
    ui.scope_builder(UiBuilder::new().max_rect(rect).layout(Layout::top_down(Align::Min)), |ui| {
        ui.set_width(w);
        add(ui)
    })
    .inner
}

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
}

fn data_path() -> PathBuf {
    directories::ProjectDirs::from("", "", "Timesheet")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
        .join("timesheet.json")
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_fonts(&cc.egui_ctx);
        setup_style(&cc.egui_ctx);
        let data_path = data_path();
        let (store, banner) = Store::open(data_path.clone());
        let today = chrono::Local::now().date_naive();
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
        };
        app.open_month(Month::of(today));
        app
    }

    fn data(&self) -> &Data {
        &self.store.data
    }

    fn save(&mut self) {
        if let Err(e) = self.store.save() {
            self.banner = Some(format!("Could not save to {}: {e}", self.data_path.display()));
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
        if path.extension().is_none_or(|e| !e.eq_ignore_ascii_case("pdf")) {
            path.set_extension("pdf");
        }
        let bytes = match report {
            Report::Detailed => pdf::detailed_report(self.data(), month),
            Report::Summary => pdf::daily_summary(self.data(), month),
        };
        let note = bytes
            .and_then(|b| std::fs::write(&path, b).map_err(|e| format!("Could not write {}: {e}", path.display())))
            .map(|()| format!("Saved {}", path.display()));
        self.export_note = Some((month, note));
    }

    fn export_note_ui(&self, ui: &mut Ui, month: Month) {
        match &self.export_note {
            Some((m, Ok(msg))) if *m == month => {
                ui.add(egui::Label::new(egui::RichText::new(msg).font(sans(13.0, W::Regular)).color(OK_FG)).wrap());
            }
            Some((m, Err(msg))) if *m == month => {
                ui.add(egui::Label::new(egui::RichText::new(msg).font(sans(13.0, W::SemiBold)).color(DANGER)).wrap());
            }
            _ => {}
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("top")
            .exact_size(64.0)
            .frame(Frame::new().fill(WHITE))
            .show(ui, |ui| self.top_bar(ui));

        let mut dirty = false;
        egui::CentralPanel::no_frame().frame(Frame::new().fill(BG)).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
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
        let full = ui.max_rect();
        ui.painter().hline(full.x_range(), full.bottom() - 0.5, Stroke::new(1.0, BORDER));
        centered(ui, 1280.0, 32.0, |ui| {
            ui.set_height(64.0);
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                let (rect, _) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::hover());
                Icon::Clock.paint(ui.painter(), rect.center(), 24.0, ACCENT, 2.0);
                text(ui, "Timesheet", sans(17.0, W::Bold), INK);
                ui.add_space(22.0);
                ui.spacing_mut().item_spacing.x = 4.0;
                for (view, label) in [(View::Timesheet, "Timesheet"), (View::PastMonths, "Past months")] {
                    let on = self.view == view;
                    let mut b = Btn::base(label).min(0.0, 40.0);
                    b.pad = 14.0;
                    b.radius = 8;
                    if on {
                        b = b.fg(ACCENT_DARK).font(sans(15.0, W::SemiBold));
                        b.fill = ACCENT_SOFT;
                        b.hover_fill = ACCENT_SOFT;
                    } else {
                        b = b.fg(INK2).font(sans(15.0, W::Regular));
                    }
                    if b.show(ui).clicked() {
                        self.view = view;
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let settings = &self.store.data.settings;
                    let (rect, response) = ui.allocate_exact_size(vec2(36.0, 36.0), Sense::click());
                    let name = settings.consultant_or_placeholder().to_owned();
                    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, format!("Signed in as {name}. Edit settings")));
                    let fill = if response.hovered() { INK2 } else { INK };
                    ui.painter().circle_filled(rect.center(), 18.0, fill);
                    ui.painter().text(rect.center(), Align2::CENTER_CENTER, settings.initials(), sans(13.0, W::SemiBold), WHITE);
                    focus_ring(ui, &response, rect, 18);
                    if response.on_hover_cursor(CursorIcon::PointingHand).on_hover_text("Settings").clicked() {
                        self.settings_draft = Some(self.store.data.settings.clone());
                    }
                });
            });
        });
    }

    fn banner_ui(&mut self, ui: &mut Ui, msg: &str) {
        Frame::new().fill(DANGER_SOFT).corner_radius(10).inner_margin(Margin::symmetric(16, 12)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.add(egui::Label::new(egui::RichText::new(msg).color(DANGER).font(sans(14.0, W::SemiBold))).wrap());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if Btn::base("").icon(Icon::Close, 16.0).a11y("Dismiss").min(32.0, 32.0).show(ui).clicked() {
                        self.banner = None;
                    }
                });
            });
        });
    }

    // -----------------------------------------------------------------------
    // Timesheet view

    /// Returns true when data changed.
    fn timesheet(&mut self, ui: &mut Ui) -> bool {
        let month = self.month;
        let weeks = month.weeks();
        self.week = self.week.min(weeks.len().saturating_sub(1));
        let mut dirty = false;

        // Title row.
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = vec2(10.0, 8.0);
                ui.horizontal(|ui| {
                    let s = &self.store.data.settings;
                    text(ui, s.project_or_placeholder(), sans(15.0, W::SemiBold), INK);
                    text(ui, s.client_or_placeholder(), sans(14.0, W::Regular), MUTED);
                    pill(ui, "Draft · sent monthly", CHIP, INK2, sans(13.0, W::SemiBold), vec2(10.0, 3.0), 99);
                });
                ui.horizontal(|ui| {
                    if Btn::outline("").icon(Icon::ChevronLeft, 18.0).a11y("Previous month").min(44.0, 44.0).show(ui).clicked() {
                        self.open_month(month.prev());
                    }
                    ui.add_space(4.0);
                    text(ui, month.label(), sans(30.0, W::Bold), INK);
                    ui.add_space(4.0);
                    if Btn::outline("").icon(Icon::ChevronRight, 18.0).a11y("Next month").min(44.0, 44.0).show(ui).clicked() {
                        self.open_month(month.next());
                    }
                });
            });
            ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                text(ui, "Weekdays only · max 8 h per day", sans(14.0, W::Regular), MUTED);
                for kind in Kind::ALL.iter().rev() {
                    // Inside a right-to-left layout: the label is added first so the swatch sits on its left.
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        text(ui, kind.label(), sans(14.0, W::Regular), INK2);
                        swatch(ui, kind_color(*kind));
                    });
                }
            });
        });

        self.week_chips(ui, &weeks);

        let wide = ui.available_width() >= 1000.0;
        let side_w = 340.0;
        let gap = 24.0;
        let main_w = if wide { ui.available_width() - side_w - gap } else { ui.available_width() };
        let mut days_ui = |ui: &mut Ui, this: &mut Self| {
            ui.set_width(main_w);
            ui.spacing_mut().item_spacing.y = 16.0;
            let mut actions = Vec::new();
            for &date in &weeks[this.week] {
                dirty |= day_card(ui, &mut this.store.data, date, &mut actions);
            }
            dirty |= !actions.is_empty();
            for action in actions {
                let data = &mut this.store.data;
                match action {
                    Action::Add(d) => data.add_line(d),
                    Action::Copy { from, to } => data.copy_day(from, to),
                    Action::Remove(id) => data.remove_line(id),
                }
            }
        };
        if wide {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.vertical(|ui| days_ui(ui, self));
                ui.vertical(|ui| {
                    ui.set_width(side_w);
                    self.sidebar(ui, &weeks);
                });
            });
        } else {
            ui.vertical(|ui| days_ui(ui, self));
            self.sidebar(ui, &weeks);
        }
        dirty
    }

    fn week_chips(&mut self, ui: &mut Ui, weeks: &[Vec<NaiveDate>]) {
        let gap = 10.0;
        let width = ui.available_width();
        let cols = (((width + gap) / 160.0).floor() as usize).clamp(1, weeks.len().max(1));
        let chip_w = (width - gap * (cols - 1) as f32) / cols as f32;
        let month = self.month;
        for row in weeks.chunks(cols).enumerate().map(|(r, c)| (r * cols, c)).collect::<Vec<_>>() {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (k, week) in row.1.iter().enumerate() {
                    let index = row.0 + k;
                    if week_chip(ui, self.data(), month, week, index == self.week, chip_w).clicked() {
                        self.week = index;
                    }
                }
            });
        }
        ui.add_space(-10.0);
    }

    fn sidebar(&mut self, ui: &mut Ui, weeks: &[Vec<NaiveDate>]) {
        let month = self.month;
        let data = self.data();
        let days = month.weekdays();
        let open: Vec<NaiveDate> = days.iter().copied().filter(|&d| data.day_total(d) < MAX_DAY_HOURS).collect();
        let any_over = data.has_overfull_day(month);
        let target = month.target_hours();
        let total = data.month_total(month);
        let by_kind: Vec<(Kind, f64)> = Kind::ALL.iter().map(|&k| (k, data.kind_total(month, k))).collect();
        ui.spacing_mut().item_spacing.y = 16.0;

        card(ui, BORDER, Margin::same(20), |ui| {
            ui.spacing_mut().item_spacing.y = 16.0;
            ui.horizontal(|ui| {
                text(ui, month.name(), sans(17.0, W::Bold), INK);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let mut job = LayoutJob::default();
                    job.append(&fmt_hours(total), 0.0, egui::TextFormat::simple(mono(22.0, true), INK));
                    job.append(&format!(" / {} h", fmt_hours(target)), 0.0, egui::TextFormat::simple(mono(14.0, false), MUTED));
                    ui.label(job);
                });
            });
            let segments: Vec<(f32, Color32)> = by_kind.iter().map(|&(k, h)| ((h / target) as f32, kind_color(k))).collect();
            bar(ui, ui.available_width(), 10.0, &segments);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                for &(k, h) in &by_kind {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        swatch(ui, kind_color(k));
                        text(ui, k.label(), sans(15.0, W::Regular), INK);
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            text(ui, format!("{} h", fmt_hours(h)), mono(15.0, false), INK);
                        });
                    });
                }
            });
            hline(ui, DIVIDER);
            ui.add_space(-2.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
                let mut job = LayoutJob::default();
                job.append(&format!("{} of {}", days.len() - open.len(), days.len()), 0.0, egui::TextFormat::simple(sans(14.0, W::Bold), INK));
                job.append(" workdays complete", 0.0, egui::TextFormat::simple(sans(14.0, W::Regular), INK2));
                ui.add(egui::Label::new(job).extend());
                if let Some(&first_open) = open.first() {
                    let label = format!("Next open day: {} {}", weekday_short(first_open), first_open.day());
                    let mut b = Btn::outline(&label).fg(ACCENT).font(sans(15.0, W::SemiBold)).min(0.0, 40.0);
                    b.pad = 12.0;
                    if b.show(ui).clicked() {
                        self.week = weeks.iter().position(|w| w.contains(&first_open)).unwrap_or(0);
                    }
                }
            });
        });

        let mut export = None;
        card(ui, BORDER, Margin::same(20), |ui| {
            ui.spacing_mut().item_spacing.y = 12.0;
            text(ui, format!("Export {}", month.name()), sans(17.0, W::Bold), INK);
            if any_over {
                text(ui, "Fix days over 8 h before exporting.", sans(14.0, W::SemiBold), DANGER);
            }
            if !open.is_empty() {
                let msg = format!("{} workdays still under 8 h. You can export a draft anytime.", open.len());
                ui.add(egui::Label::new(egui::RichText::new(msg).font(sans(14.0, W::Regular)).color(WARN)).wrap());
            }
            let full = ui.available_width();
            for (report, primary) in [(Report::Detailed, true), (Report::Summary, false)] {
                let label = format!("{} (PDF)", report.title());
                let mut b = if primary { Btn::primary(&label) } else { Btn::outline(&label).font(sans(15.0, W::SemiBold)) };
                b = b.icon(Icon::Download, 18.0).min(full, 48.0);
                b.left = true;
                b.enabled = !any_over;
                if b.show(ui).clicked() {
                    export = Some(report);
                }
            }
            let note = format!("Both PDFs cover {}, weekdays only.", month.period_label().rsplit_once(',').map_or("", |p| p.0));
            text(ui, note, sans(13.0, W::Regular), MUTED);
            self.export_note_ui(ui, month);
        });
        if let Some(report) = export {
            self.export(month, report);
        }
    }

    // -----------------------------------------------------------------------
    // Past months view

    /// Returns true when data changed.
    fn past_months(&mut self, ui: &mut Ui) -> bool {
        let today = self.today;
        let current = Month::of(today);
        let months = self.data().closed_months(current);
        let statuses: Vec<ApprovalStatus> = months.iter().map(|&m| self.data().approval(m).status(today)).collect();
        let mut dirty = false;

        // Title row.
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = vec2(10.0, 6.0);
                ui.horizontal(|ui| {
                    let s = &self.store.data.settings;
                    text(ui, s.project_or_placeholder(), sans(15.0, W::SemiBold), INK);
                    text(ui, s.client_or_placeholder(), sans(14.0, W::Regular), MUTED);
                });
                text(ui, "Past months", sans(30.0, W::Bold), INK);
            });
            ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                let count = |f: fn(&ApprovalStatus) -> bool| statuses.iter().filter(|s| f(s)).count();
                let counts = [
                    (count(|s| matches!(s, ApprovalStatus::NotSent)), "not sent", CHIP, INK2),
                    (count(|s| matches!(s, ApprovalStatus::Awaiting { .. })), "awaiting signature", ACCENT_SOFT, ACCENT_DARK),
                    (count(|s| matches!(s, ApprovalStatus::Approved)), "approved", OK_SOFT, OK_FG),
                ];
                for (n, label, bg, fg) in counts.into_iter().rev() {
                    pill(ui, &format!("{n} {label}"), bg, fg, sans(14.0, W::SemiBold), vec2(12.0, 8.0), 10);
                }
            });
        });

        if self.current_month_card(ui, current) {
            self.open_month(current);
            self.view = View::Timesheet;
        }

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(14.0, 6.0);
            text(ui, "Each month:", sans(14.0, W::SemiBold), INK);
            let steps = [
                "1. Export both PDFs",
                "2. Tick “Sent” when emailed to your manager",
                "3. Tick “Approved” when they reply",
                "4. Attach the signed PDF",
            ];
            for (i, step) in steps.into_iter().enumerate() {
                if i > 0 {
                    text(ui, "›", sans(14.0, W::Regular), INK2);
                }
                text(ui, step, sans(14.0, W::Regular), INK2);
            }
        });

        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 12.0;
            if months.is_empty() {
                text(ui, "No closed months yet. A month appears here once it ends.", sans(15.0, W::Regular), MUTED);
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
        let data = self.data();
        let total = data.month_total(month);
        let target = month.target_hours();
        let (status, bg, fg) = progress_status(data, month);
        let frame = Frame::new().fill(WHITE).stroke(Stroke::new(1.0, BORDER)).corner_radius(14).inner_margin(Margin::symmetric(20, 16));
        let rect = frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                ui.horizontal(|ui| {
                    ui.set_min_width(150.0);
                    text(ui, month.label(), sans(17.0, W::Bold), INK);
                });
                pill(ui, status, bg, fg, sans(13.0, W::SemiBold), vec2(10.0, 4.0), 99);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    text(ui, "Continue entering time", sans(15.0, W::SemiBold), ACCENT);
                    text(ui, format!("{} / {} h", fmt_hours(total), fmt_hours(target)), mono(14.0, false), INK);
                    ui.spacing_mut().item_spacing.x = 10.0;
                    bar(ui, ui.available_width(), 8.0, &[((total / target) as f32, ACCENT)]);
                });
            });
        })
        .response
        .rect;
        let response = ui.interact(rect, ui.id().with("current-month"), Sense::click());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Link, true, format!("{}, continue entering time", month.label())));
        if response.hovered() {
            ui.painter().rect_stroke(rect, 14, Stroke::new(1.0, HOVER_BORDER), StrokeKind::Inside);
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
        let data = self.data();
        let today = self.today;
        let label = month.label();
        let can_export = !data.has_overfull_day(month);
        let (status_text, pill_bg, pill_fg, border) = match status {
            ApprovalStatus::NotSent => ("Not sent", CHIP, INK2, BORDER),
            ApprovalStatus::Awaiting { late: true, .. } => ("Follow up", WARN_SOFT, WARN, WARN_BORDER),
            ApprovalStatus::Awaiting { .. } => ("Awaiting signature", ACCENT_SOFT, ACCENT_DARK, BORDER),
            ApprovalStatus::Approved if approval.signed_pdf.is_some() => ("Approved · signed copy saved", OK_SOFT, OK_FG, BORDER),
            ApprovalStatus::Approved => ("Approved", OK_SOFT, OK_FG, BORDER),
        };
        // Mockup flex bases, except the first column, widened to fit the "Approved · signed copy saved" pill.
        let columns = [(200.0, 1.0), (170.0, 1.0), (190.0, 1.0), (190.0, 1.0), (250.0, 1.4)];
        card(ui, border, Margin::symmetric(20, 18), |ui| {
            flex_wrap(ui, &columns, vec2(28.0, 20.0), |ui, column| {
                ui.spacing_mut().item_spacing.y = 6.0;
                match column {
                    0 => {
                        text(ui, &label, sans(17.0, W::Bold), INK);
                        let mut job = LayoutJob::default();
                        job.append(&format!("{} h", fmt_hours(data.month_total(month))), 0.0, egui::TextFormat::simple(mono(14.0, true), INK));
                        let days = format!(" · {} workdays", month.weekdays().len());
                        job.append(&days, 0.0, egui::TextFormat::simple(sans(14.0, W::Regular), INK2));
                        ui.label(job);
                        pill(ui, status_text, pill_bg, pill_fg, sans(13.0, W::SemiBold), vec2(10.0, 4.0), 99);
                    }
                    1 => {
                        caption(ui, "Generated PDFs");
                        ui.spacing_mut().item_spacing.y = 0.0;
                        for report in [Report::Detailed, Report::Summary] {
                            let mut b = Btn::base(report.title()).icon(Icon::Download, 16.0).fg(ACCENT).min(0.0, 32.0);
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
                            text(ui, "Fix days over 8 h to export.", sans(13.0, W::SemiBold), DANGER);
                        }
                        self.export_note_ui(ui, month);
                    }
                    2 => {
                        let mut sent = approval.sent.is_some();
                        if checkbox(ui, &mut sent, "Sent to manager", ACCENT, true).changed() {
                            approval.set_sent(sent, today);
                        }
                        match approval.sent.as_mut() {
                            Some(date) => {
                                date_field(ui, date, &format!("sent-{}", month.key()), &format!("Date sent, {label}"));
                            }
                            None => {
                                text(ui, "Tick after emailing both PDFs.", sans(13.0, W::Regular), MUTED);
                            }
                        }
                    }
                    3 => {
                        let mut approved = approval.approved.is_some();
                        let enabled = approval.sent.is_some();
                        if checkbox(ui, &mut approved, "Approved by manager", OK, enabled).changed() {
                            approval.set_approved(approved, today);
                        }
                        if let Some(date) = approval.approved.as_mut() {
                            date_field(ui, date, &format!("approved-{}", month.key()), &format!("Date approved, {label}"));
                        } else if let ApprovalStatus::Awaiting { days, late } = status {
                            let ago = match days {
                                0 => "Sent today".to_owned(),
                                1 => "Sent 1 day ago".to_owned(),
                                n => format!("Sent {n} days ago"),
                            };
                            let (tail, color) = if late { ("time to follow up", WARN) } else { ("waiting for reply", INK2) };
                            text(ui, format!("{ago} · {tail}"), sans(13.0, W::SemiBold), color);
                        }
                    }
                    _ => {
                        caption(ui, "Signed timesheet");
                        if let Some(name) = &approval.signed_pdf {
                            signed_file_row(ui, name, &label, action);
                        } else {
                            let mut b = Btn::base("Attach signed PDF")
                                .icon(Icon::Paperclip, 18.0)
                                .fg(ACCENT)
                                .font(sans(15.0, W::SemiBold))
                                .min(ui.available_width(), 44.0)
                                .a11y(format!("Attach signed PDF, {label}"));
                            b.dashed = true;
                            b.left = true;
                            b.pad = 14.0;
                            b.hover_fill = ACCENT_TINT;
                            if b.show(ui).clicked() {
                                *action = Some(MonthAction::Attach);
                            }
                            let hint = if status == ApprovalStatus::Approved {
                                "Optional · keeps a copy with this month."
                            } else {
                                "Attaching marks the month as sent and approved."
                            };
                            text(ui, hint, sans(13.0, W::Regular), MUTED);
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
                self.store.attach_signed_pdf(month, &path, self.today).map_err(|e| format!("Could not attach {}: {e}", path.display()))
            }
            MonthAction::Open => {
                let path = self.store.signed_pdf_path(month);
                if let Err(e) = open_path(&path) {
                    self.banner = Some(format!("Could not open {}: {e}", path.display()));
                }
                return false;
            }
            MonthAction::Remove => self.store.remove_signed_pdf(month).map_err(|e| format!("Could not remove the signed PDF: {e}")),
        };
        match result {
            Ok(()) => true,
            Err(msg) => {
                self.banner = Some(msg);
                false
            }
        }
    }

    // -----------------------------------------------------------------------
    // Settings

    /// Returns true when settings were saved.
    fn settings_modal(&mut self, ctx: &egui::Context) -> bool {
        let Some(draft) = self.settings_draft.as_mut() else { return false };
        let mut save = false;
        let mut cancel = false;
        let frame = Frame::new().fill(WHITE).corner_radius(14).inner_margin(Margin::same(24)).stroke(Stroke::new(1.0, BORDER));
        let modal = egui::Modal::new(Id::new("settings")).frame(frame).show(ctx, |ui| {
            ui.set_width(440.0);
            ui.spacing_mut().item_spacing.y = 6.0;
            text(ui, "Settings", sans(20.0, W::Bold), INK);
            text(ui, "Shown in the app header and on both PDF exports.", sans(14.0, W::Regular), MUTED);
            ui.add_space(10.0);
            for (label, value, hint) in [
                ("Consultant name", &mut draft.consultant, "e.g. Jane Doe"),
                ("Client name", &mut draft.client, "e.g. Acme Corp"),
                ("Project name", &mut draft.project, "e.g. Website redesign"),
            ] {
                text(ui, label, sans(14.0, W::SemiBold), INK2);
                ui.add(
                    TextEdit::singleline(value)
                        .hint_text(hint)
                        .font(sans(15.0, W::Regular))
                        .text_color(INK)
                        .margin(Margin::symmetric(12, 0))
                        .vertical_align(Align::Center)
                        .min_size(vec2(0.0, 44.0))
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(8.0);
            }
            ui.add(
                egui::Label::new(
                    egui::RichText::new(format!("Data is saved automatically in {}", self.data_path.display()))
                        .font(sans(13.0, W::Regular))
                        .color(MUTED),
                )
                .wrap(),
            );
            ui.add_space(10.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if Btn::primary("Save").min(96.0, 44.0).show(ui).clicked() {
                    save = true;
                }
                if Btn::outline("Cancel").show(ui).clicked() {
                    cancel = true;
                }
            });
        });
        if save {
            let draft = self.settings_draft.take().unwrap_or_default();
            self.store.data.settings = Settings {
                consultant: draft.consultant.trim().to_owned(),
                client: draft.client.trim().to_owned(),
                project: draft.project.trim().to_owned(),
            };
            return true;
        }
        if cancel || modal.should_close() {
            self.settings_draft = None;
        }
        false
    }
}

// ---------------------------------------------------------------------------
// Week chip and day card

fn week_chip(ui: &mut Ui, data: &Data, month: Month, week: &[NaiveDate], selected: bool, width: f32) -> Response {
    let total: f64 = week.iter().map(|&d| data.day_total(d)).sum();
    let target = week.len() as f64 * MAX_DAY_HOURS;
    let over = week.iter().any(|&d| data.day_total(d) > MAX_DAY_HOURS);
    let (status, status_color, bar_color) = if over {
        ("Over", DANGER, DANGER)
    } else if total >= target {
        ("Done", OK_FG, OK)
    } else if total > 0.0 {
        ("In progress", MUTED, ACCENT)
    } else {
        ("Empty", MUTED, ACCENT)
    };
    let label = format!("{} {}", month.short_name(), day_range(week, "–"));

    let (rect, response) = ui.allocate_exact_size(vec2(width, 64.0), Sense::click());
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, selected, format!("{label}, {status}")));
    let p = ui.painter();
    let (fill, stroke) = if selected {
        (ACCENT_TINT, Stroke::new(2.0, ACCENT))
    } else if response.hovered() {
        (WHITE, Stroke::new(1.0, HOVER_BORDER))
    } else {
        (WHITE, Stroke::new(1.0, BORDER))
    };
    p.rect_filled(rect, 12, fill);
    p.rect_stroke(rect, 12, stroke, StrokeKind::Inside);
    focus_ring(ui, &response, rect, 12);
    let inner = rect.shrink2(vec2(14.0, 10.0));
    p.text(inner.left_top(), Align2::LEFT_TOP, &label, sans(15.0, W::SemiBold), INK);
    p.text(pos2(inner.right(), inner.top() + 2.0), Align2::RIGHT_TOP, status, sans(12.0, W::SemiBold), status_color);
    let numbers = format!("{}/{}", fmt_hours(total), fmt_hours(target));
    let numbers_rect = p.text(inner.right_bottom(), Align2::RIGHT_BOTTOM, numbers, mono(13.0, false), INK2);
    let bar_rect = Rect::from_min_max(
        pos2(inner.left(), numbers_rect.center().y - 3.0),
        pos2(numbers_rect.left() - 8.0, numbers_rect.center().y + 3.0),
    );
    paint_bar(p, bar_rect, BORDER, &[((total / target).min(1.0) as f32, bar_color)]);
    response.on_hover_cursor(CursorIcon::PointingHand)
}

/// One weekday card with its lines. Edits apply directly; structural changes are queued in `actions`.
/// Returns true when a line was edited.
fn day_card(ui: &mut Ui, data: &mut Data, date: NaiveDate, actions: &mut Vec<Action>) -> bool {
    let total = data.day_total(date);
    let left = MAX_DAY_HOURS - total;
    let over = total > MAX_DAY_HOURS;
    let next_hint = format!("{} {} h", data.next_kind(date).label(), fmt_hours(left.max(0.0)));
    let prev = data.previous_filled_day(date);
    let indexes: Vec<usize> = data.entries.iter().enumerate().filter(|(_, e)| e.date == date).map(|(i, _)| i).collect();
    let segments: Vec<(f32, Color32)> =
        Kind::ALL.iter().map(|&k| ((data.day_kind_total(date, k) / MAX_DAY_HOURS) as f32, kind_color(k))).collect();
    let (status, pill_bg, pill_fg) = if over {
        ("Over 8 h".to_owned(), DANGER_SOFT, DANGER)
    } else if left <= 0.0 {
        ("Complete".to_owned(), OK_SOFT, OK_FG)
    } else {
        (format!("{} h left", fmt_hours(left)), CHIP, INK2)
    };
    let day_name = weekday_long(date);
    let mut dirty = false;

    let border = if over { DANGER } else { BORDER };
    card(ui, border, Margin::ZERO, |ui| {
        ui.spacing_mut().item_spacing = vec2(12.0, 0.0);
        // Header.
        Frame::new().inner_margin(Margin::symmetric(20, 14)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                ui.horizontal(|ui| {
                    ui.set_min_width(140.0);
                    ui.spacing_mut().item_spacing.x = 5.0;
                    text(ui, weekday_short(date), sans(17.0, W::Bold), INK);
                    text(ui, format!("{} {}", Month::of(date).short_name(), date.day()), sans(17.0, W::Medium), MUTED);
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    pill(ui, &status, pill_bg, pill_fg, sans(13.0, W::SemiBold), vec2(10.0, 4.0), 99);
                    text(ui, format!("{} / 8 h", fmt_hours(total)), mono(15.0, true), INK);
                    bar(ui, ui.available_width(), 8.0, &segments);
                });
            });
        });
        hline(ui, DIVIDER);

        if indexes.is_empty() {
            Frame::new().inner_margin(Margin::symmetric(20, 18)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    text(ui, "No time recorded yet.", sans(15.0, W::Regular), MUTED);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if Btn::primary(&format!("Add line · {next_hint}")).show(ui).clicked() {
                            actions.push(Action::Add(date));
                        }
                        if let Some(from) = prev {
                            let name = if Month::of(from) == Month::of(date) {
                                format!("{} {}", weekday_short(from), from.day())
                            } else {
                                format!("{} {} {}", weekday_short(from), Month::of(from).short_name(), from.day())
                            };
                            if Btn::outline(&format!("Copy {name}")).show(ui).clicked() {
                                actions.push(Action::Copy { from, to: date });
                            }
                        }
                    });
                });
            });
            return;
        }

        for (k, &i) in indexes.iter().enumerate() {
            if k > 0 {
                hline(ui, DIVIDER);
            }
            let line_no = k + 1;
            let entry = &mut data.entries[i];
            Frame::new().inner_margin(Margin { left: 20, right: 12, top: 10, bottom: 10 }).show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Task type toggle.
                    Frame::new().fill(CHIP).corner_radius(10).inner_margin(Margin::same(3)).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            for kind in Kind::ALL {
                                let on = entry.kind == kind;
                                let mut b = Btn::base(kind.label()).min(0.0, 38.0).font(sans(14.0, if on { W::SemiBold } else { W::Medium }));
                                b.pad = 12.0;
                                b.radius = 8;
                                b.a11y = Some(format!("{}, {day_name} line {line_no}", kind.label()));
                                if on {
                                    b.fill = kind_toggle_fill(kind);
                                    b.hover_fill = b.fill;
                                    b.fg = WHITE;
                                } else {
                                    b.hover_fill = BORDER;
                                    b.fg = INK2;
                                }
                                if b.show(ui).clicked() && !on {
                                    entry.kind = kind;
                                    dirty = true;
                                }
                            }
                        });
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let remove = Btn::base("")
                            .icon(Icon::Close, 16.0)
                            .fg(MUTED)
                            .min(44.0, 44.0)
                            .a11y(format!("Remove {day_name} line {line_no}"))
                            .show(ui);
                        if remove.clicked() {
                            actions.push(Action::Remove(entry.id));
                        }
                        ui.spacing_mut().item_spacing.x = 6.0;
                        text(ui, "h", sans(15.0, W::Regular), MUTED);
                        ui.scope(|ui| {
                            if over {
                                let widgets = &mut ui.visuals_mut().widgets;
                                widgets.inactive.bg_stroke = Stroke::new(1.0, DANGER);
                                widgets.hovered.bg_stroke = Stroke::new(1.0, DANGER);
                            }
                            let hours = ui.add(
                                TextEdit::singleline(&mut entry.hours)
                                    .id_salt(("hours", entry.id))
                                    .font(mono(15.0, false))
                                    .text_color(INK)
                                    .horizontal_align(Align::RIGHT)
                                    .vertical_align(Align::Center)
                                    .margin(Margin { left: 12, right: 8, top: 0, bottom: 0 })
                                    .min_size(vec2(76.0, 44.0))
                                    .desired_width(56.0)
                                    .char_limit(5),
                            );
                            hours.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, format!("Hours, {day_name} line {line_no}")));
                            dirty |= hours.changed();
                        });
                        ui.spacing_mut().item_spacing.x = 12.0;
                        let desc = ui.add(
                            TextEdit::singleline(&mut entry.desc)
                                .id_salt(("desc", entry.id))
                                .hint_text("What did you do? e.g. ticket #1234")
                                .font(sans(15.0, W::Regular))
                                .text_color(INK)
                                .vertical_align(Align::Center)
                                .margin(Margin::symmetric(12, 0))
                                .min_size(vec2(0.0, 44.0))
                                .desired_width(ui.available_width()),
                        );
                        desc.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, format!("Description, {day_name} line {line_no}")));
                        dirty |= desc.changed();
                    });
                });
            });
        }

        // Footer, only when it has something to show.
        if left <= 0.0 && !over {
            return;
        }
        Frame::new().inner_margin(Margin { left: 20, right: 20, top: 10, bottom: 14 }).show(ui, |ui| {
            ui.horizontal(|ui| {
                if left > 0.0 {
                    let label = format!("Add line · {next_hint}");
                    let mut b = Btn::base(&label).icon(Icon::Plus, 16.0).fg(ACCENT).font(sans(15.0, W::SemiBold));
                    b.dashed = true;
                    b.pad = 14.0;
                    b.hover_fill = ACCENT_TINT;
                    if b.show(ui).clicked() {
                        actions.push(Action::Add(date));
                    }
                }
                if over {
                    let msg = format!("A day cannot exceed 8 h. Remove {} h.", fmt_hours(total - MAX_DAY_HOURS));
                    text(ui, msg, sans(15.0, W::SemiBold), DANGER);
                }
            });
        });
    });
    dirty
}

/// Saved signed copy: the name opens it, the cross removes it.
fn signed_file_row(ui: &mut Ui, name: &str, month_label: &str, action: &mut Option<MonthAction>) {
    let frame = Frame::new()
        .fill(OK_TINT)
        .stroke(Stroke::new(1.0, OK_BORDER))
        .corner_radius(10)
        .inner_margin(Margin { left: 12, right: 6, top: 6, bottom: 6 });
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let (rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
            Icon::FileCheck.paint(ui.painter(), rect.center(), 18.0, OK_FG, 2.0);
            // Leaves room for the item spacing before the name and the remove button.
            let name_w = (ui.available_width() - 8.0 - 8.0 - 36.0).max(0.0);
            ui.allocate_ui_with_layout(vec2(name_w, 36.0), Layout::left_to_right(Align::Center), |ui| {
                ui.set_width(name_w);
                let rich = egui::RichText::new(name).font(sans(14.0, W::SemiBold)).color(OK_FG);
                let link = ui.add(egui::Label::new(rich).truncate().sense(Sense::click()));
                link.widget_info(|| WidgetInfo::labeled(WidgetType::Link, true, format!("Open {name}")));
                if link.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                    *action = Some(MonthAction::Open);
                }
            });
            let mut b = Btn::base("").icon(Icon::Close, 14.0).fg(INK2).min(36.0, 36.0).a11y(format!("Remove signed PDF for {month_label}"));
            b.radius = 8;
            b.pad = 0.0;
            if b.show(ui).clicked() {
                *action = Some(MonthAction::Remove);
            }
        });
    });
}
