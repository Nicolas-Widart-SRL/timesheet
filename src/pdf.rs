//! The two monthly exports: detailed report (one row per line) and daily summary (one row per weekday).
//!
//! Layout is computed into a list of drawing operations per page, then rendered with krilla.

use chrono::Datelike;
use krilla::color::rgb;
use krilla::geom::{PathBuilder, Point, Rect};
use krilla::metadata::Metadata;
use krilla::num::NormalizedF32;
use krilla::page::PageSettings;
use krilla::paint::{Fill, FillRule, Stroke};
use krilla::text::{Font, TextDirection};
use krilla::Document;

use crate::fonts;
use crate::model::{day_range, fmt_report, weekday_short, Data, Kind, Month, MAX_DAY_HOURS};

// A4 in points.
const PAGE_W: f32 = 595.28;
const PAGE_H: f32 = 841.89;
const LEFT: f32 = 48.0;
const RIGHT: f32 = PAGE_W - 48.0;
const TOP_NEXT_PAGE: f32 = 40.0;
const BOTTOM: f32 = PAGE_H - 44.0;

const ROW_H: f32 = 22.3;
const SUMMARY_ROW_H: f32 = 22.0;
const BODY: f32 = 10.0;

type Rgb = (u8, u8, u8);
const INK: Rgb = (0x15, 0x17, 0x1C);
const INK2: Rgb = (0x3F, 0x44, 0x52);
const MUTED: Rgb = (0x5B, 0x61, 0x70);
const LINE: Rgb = (0xE2, 0xE4, 0xE9);
const SOFT: Rgb = (0xF4, 0xF5, 0xF7);

pub fn kind_rgb(kind: Kind) -> Rgb {
    match kind {
        Kind::Frontend => (0x24, 0x46, 0xD8),
        Kind::Backend => (0xC2, 0x57, 0x1A),
        Kind::Support => (0x0F, 0x7B, 0x6C),
    }
}

#[derive(Clone, Copy)]
enum Face {
    Regular,
    SemiBold,
    Bold,
    Mono,
    MonoSemiBold,
}

impl Face {
    const ALL: [Face; 5] = [Face::Regular, Face::SemiBold, Face::Bold, Face::Mono, Face::MonoSemiBold];

    fn bytes(self) -> &'static [u8] {
        match self {
            Face::Regular => fonts::SANS_REGULAR,
            Face::SemiBold => fonts::SANS_SEMIBOLD,
            Face::Bold => fonts::SANS_BOLD,
            Face::Mono => fonts::MONO_REGULAR,
            Face::MonoSemiBold => fonts::MONO_SEMIBOLD,
        }
    }

    /// Width of `text` in points, from the font's horizontal advances.
    fn width(self, text: &str, size: f32) -> f32 {
        let face = ttf_parser::Face::parse(self.bytes(), 0).expect("embedded font parses");
        let units: u32 = text
            .chars()
            .map(|c| face.glyph_index(c).and_then(|g| face.glyph_hor_advance(g)).unwrap_or(0) as u32)
            .sum();
        units as f32 * size / face.units_per_em() as f32
    }
}

enum Op {
    Text { x: f32, y: f32, face: Face, size: f32, color: Rgb, text: String },
    Rect { x: f32, y: f32, w: f32, h: f32, radius: f32, fill: Option<Rgb>, stroke: Option<(Rgb, f32)> },
    HLine { x1: f32, x2: f32, y: f32, color: Rgb, width: f32 },
}

#[derive(Clone, Copy)]
enum Align {
    Left,
    Right,
}

/// Page-aware layout cursor.
struct Canvas {
    pages: Vec<Vec<Op>>,
    y: f32,
}

impl Canvas {
    fn new() -> Self {
        Canvas { pages: vec![Vec::new()], y: 0.0 }
    }

    fn push(&mut self, op: Op) {
        self.pages.last_mut().expect("at least one page").push(op);
    }

    fn at_page_top(&self) -> bool {
        self.y <= TOP_NEXT_PAGE
    }

    /// Starts a new page unless `height` still fits on the current one.
    fn ensure(&mut self, height: f32) {
        if self.y + height > BOTTOM && !self.at_page_top() {
            self.pages.push(Vec::new());
            self.y = TOP_NEXT_PAGE;
        }
    }

    fn text(&mut self, x: f32, baseline: f32, align: Align, face: Face, size: f32, color: Rgb, text: &str) {
        let x = match align {
            Align::Left => x,
            Align::Right => x - face.width(text, size),
        };
        self.push(Op::Text { x, y: baseline, face, size, color, text: text.to_owned() });
    }

    /// Text with extra tracking between characters (used for the small uppercase eyebrow).
    fn tracked(&mut self, x: f32, baseline: f32, face: Face, size: f32, color: Rgb, text: &str, tracking: f32) {
        let mut x = x;
        let mut buf = [0u8; 4];
        for c in text.chars() {
            let s = c.encode_utf8(&mut buf);
            self.text(x, baseline, Align::Left, face, size, color, s);
            x += face.width(s, size) + tracking;
        }
    }

    fn hline(&mut self, x1: f32, x2: f32, y: f32, color: Rgb, width: f32) {
        self.push(Op::HLine { x1, x2, y, color, width });
    }

    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, fill: Option<Rgb>, stroke: Option<(Rgb, f32)>) {
        self.push(Op::Rect { x, y, w, h, radius, fill, stroke });
    }

    /// Small rounded square marking a task type.
    fn swatch(&mut self, x: f32, baseline: f32, color: Rgb) {
        let s = 5.4;
        self.rect(x, baseline - 6.6, s, s, 1.2, Some(color), None);
    }
}

fn header(c: &mut Canvas, data: &Data, month: Month, eyebrow: &str) {
    let s = &data.settings;
    c.tracked(LEFT, 63.0, Face::SemiBold, 8.6, INK2, eyebrow, 0.55);
    c.text(LEFT, 92.0, Align::Left, Face::Bold, 22.5, INK, &month.label());
    let subtitle = format!("{} · {}", s.project_or_placeholder(), s.client_or_placeholder());
    c.text(LEFT, 111.0, Align::Left, Face::Regular, 10.5, INK2, &subtitle);

    let rows = [
        ("Consultant", s.consultant_or_placeholder().to_owned()),
        ("Period", month.period_label()),
        ("Workdays", month.weekdays().len().to_string()),
    ];
    let value_w = rows.iter().map(|(_, v)| Face::SemiBold.width(v, BODY)).fold(0.0, f32::max);
    let label_right = RIGHT - value_w.max(52.0) - 12.0;
    for (i, (label, value)) in rows.iter().enumerate() {
        let y = 65.0 + i as f32 * 15.5;
        c.text(label_right, y, Align::Right, Face::Regular, BODY, MUTED, label);
        c.text(RIGHT, y, Align::Right, Face::SemiBold, BODY, INK, value);
    }
    c.hline(LEFT, RIGHT, 128.0, INK, 1.4);
    c.y = 128.0;
}

/// Signature lines `gap_above` below the content; the gap shrinks (down to 40 pt) to stay on the same page.
fn signatures(c: &mut Canvas, gap_above: f32) {
    let block = 26.0;
    let gap = gap_above.min(BOTTOM - block - c.y).max(40.0);
    c.ensure(gap + block);
    let y = if c.at_page_top() { c.y + 40.0 } else { c.y + gap };
    let column_gap = 31.0;
    let w = (RIGHT - LEFT - column_gap) / 2.0;
    for (i, label) in ["Consultant signature · date", "Client approval · date"].into_iter().enumerate() {
        let x = LEFT + i as f32 * (w + column_gap);
        c.hline(x, x + w, y, INK2, 0.8);
        c.text(x, y + 14.0, Align::Left, Face::Regular, BODY, INK2, label);
    }
    c.y = y + 26.0;
}

/// PDF 1 · Detailed report: every line grouped by week and day.
pub fn detailed_report(data: &Data, month: Month) -> Result<Vec<u8>, String> {
    let mut c = Canvas::new();
    header(&mut c, data, month, "TIMESHEET · DETAILED REPORT");

    // Totals tiles.
    let top = 148.0;
    let h = 51.0;
    let cell_w = (RIGHT - LEFT) / 4.0;
    c.rect(LEFT + 3.0 * cell_w, top, cell_w, h, 0.0, Some(SOFT), None);
    let tiles = Kind::ALL
        .iter()
        .map(|&k| (k.label(), kind_rgb(k), data.kind_total(month, k)))
        .chain(std::iter::once(("Total", INK, data.month_total(month))));
    for (i, (label, color, hours)) in tiles.enumerate() {
        let x = LEFT + i as f32 * cell_w;
        if i > 0 {
            c.push(Op::Rect { x, y: top, w: 0.0, h, radius: 0.0, fill: None, stroke: Some((LINE, 0.7)) });
        }
        c.swatch(x + 11.0, top + 17.5, color);
        c.text(x + 19.5, top + 17.5, Align::Left, Face::Regular, BODY, INK2, label);
        c.text(x + 11.0, top + 38.5, Align::Left, Face::MonoSemiBold, 14.5, INK, &format!("{} h", fmt_report(hours)));
    }
    c.rect(LEFT, top, RIGHT - LEFT, h, 3.0, None, Some((LINE, 0.7)));

    // Columns.
    let x_date = LEFT + 6.0;
    let x_type = LEFT + 74.0;
    let x_desc = LEFT + 149.0;
    let x_hours = LEFT + 444.0;
    let x_day = RIGHT - 6.0;
    let desc_max = x_hours - 40.0 - x_desc;

    c.y = 218.5;
    c.hline(LEFT, RIGHT, c.y, INK, 0.8);
    let base = c.y + 15.0;
    c.text(x_date, base, Align::Left, Face::SemiBold, BODY, INK, "Date");
    c.text(x_type, base, Align::Left, Face::SemiBold, BODY, INK, "Type");
    c.text(x_desc, base, Align::Left, Face::SemiBold, BODY, INK, "Description");
    c.text(x_hours, base, Align::Right, Face::SemiBold, BODY, INK, "Hours");
    c.text(x_day, base, Align::Right, Face::SemiBold, BODY, INK, "Day");
    c.y += 24.0;
    c.hline(LEFT, RIGHT, c.y, INK, 0.8);

    for week in month.weeks() {
        let week_total: f64 = week.iter().map(|&d| data.day_total(d)).sum();
        let first_lines = data.lines(week[0]).count().max(1);
        c.ensure(ROW_H * (1 + first_lines) as f32);
        let base = c.y + 14.0;
        let label = format!("{} {}", month.short_name(), day_range(&week, " – "));
        c.text(x_date, base, Align::Left, Face::SemiBold, BODY, INK, &label);
        c.text(x_day, base, Align::Right, Face::MonoSemiBold, BODY, INK, &fmt_report(week_total));
        c.y += ROW_H;
        c.hline(LEFT, RIGHT, c.y, INK, 0.8);

        for (i, &date) in week.iter().enumerate() {
            let lines: Vec<_> = data.lines(date).collect();
            c.ensure(ROW_H * lines.len().max(1) as f32);
            if i > 0 && !c.at_page_top() {
                c.hline(LEFT, RIGHT, c.y, LINE, 0.6);
            }
            let base = c.y + 14.0;
            c.text(x_date, base, Align::Left, Face::SemiBold, BODY, INK, &format!("{} {}", weekday_short(date), date.day()));
            c.text(x_day, base, Align::Right, Face::MonoSemiBold, BODY, INK, &fmt_report(data.day_total(date)));
            if lines.is_empty() {
                c.text(x_desc, base, Align::Left, Face::Regular, BODY, MUTED, "No time recorded");
                c.y += ROW_H;
            }
            for line in lines {
                let base = c.y + 14.0;
                c.swatch(x_type, base, kind_rgb(line.kind));
                c.text(x_type + 10.5, base, Align::Left, Face::Regular, BODY, INK, line.kind.label());
                c.text(x_desc, base, Align::Left, Face::Regular, BODY, INK, &ellipsize(&line.desc, Face::Regular, BODY, desc_max));
                c.text(x_hours, base, Align::Right, Face::Mono, BODY, INK, &fmt_report(line.hours()));
                c.y += ROW_H;
            }
        }
    }

    total_row(&mut c, &format!("Total {}", month.label()), &[(x_day, data.month_total(month))], x_date);
    signatures(&mut c, 71.0);
    render(c, &format!("Timesheet · Detailed report · {}", month.label()))
}

/// PDF 2 · Daily summary: one row per weekday with hours per task type.
pub fn daily_summary(data: &Data, month: Month) -> Result<Vec<u8>, String> {
    let mut c = Canvas::new();
    header(&mut c, data, month, "TIMESHEET · DAILY SUMMARY");

    let width = RIGHT - LEFT;
    let x_date = LEFT + 8.0;
    let x_kinds = [LEFT + width * 0.43, LEFT + width * 0.639, LEFT + width * 0.843];
    let total_bg = LEFT + width * 0.858;
    let x_total = RIGHT - 8.0;
    let total_col = |c: &mut Canvas, h: f32| c.rect(total_bg, c.y, RIGHT - total_bg, h, 0.0, Some(SOFT), None);

    c.y = 146.0;
    let head_h = 24.6;
    total_col(&mut c, head_h);
    c.hline(LEFT, RIGHT, c.y, INK, 0.8);
    let base = c.y + 15.5;
    c.text(x_date, base, Align::Left, Face::SemiBold, BODY, INK, "Date");
    for (k, x) in Kind::ALL.iter().zip(x_kinds) {
        c.text(x, base, Align::Right, Face::SemiBold, BODY, INK, k.label());
        c.swatch(x - Face::SemiBold.width(k.label(), BODY) - 10.0, base, kind_rgb(*k));
    }
    c.text(x_total, base, Align::Right, Face::SemiBold, BODY, INK, "Total");
    c.y += head_h;
    c.hline(LEFT, RIGHT, c.y, INK, 0.8);

    for (w, week) in month.weeks().iter().enumerate() {
        for (i, &date) in week.iter().enumerate() {
            c.ensure(SUMMARY_ROW_H);
            total_col(&mut c, SUMMARY_ROW_H);
            if (w > 0 || i > 0) && !c.at_page_top() {
                let (color, width) = if i == 0 { (INK, 0.8) } else { (LINE, 0.6) };
                c.hline(LEFT, RIGHT, c.y, color, width);
            }
            let base = c.y + 14.0;
            let label = format!("{}, {} {}", weekday_short(date), month.short_name(), date.day());
            c.text(x_date, base, Align::Left, Face::Regular, BODY, INK, &label);
            for (k, x) in Kind::ALL.iter().zip(x_kinds) {
                c.text(x, base, Align::Right, Face::Mono, BODY, INK, &dash_if_zero(data.day_kind_total(date, *k)));
            }
            c.text(x_total, base, Align::Right, Face::MonoSemiBold, BODY, INK, &dash_if_zero(data.day_total(date)));
            c.y += SUMMARY_ROW_H;
        }
    }

    let totals: Vec<(f32, f64)> = Kind::ALL
        .iter()
        .zip(x_kinds)
        .map(|(k, x)| (x, data.kind_total(month, *k)))
        .chain(std::iter::once((x_total, data.month_total(month))))
        .collect();
    c.ensure(29.0 + 40.0);
    total_col(&mut c, 29.0);
    total_row(&mut c, "Total", &totals, x_date);

    let note = format!(
        "Hours in decimal, weekdays only, maximum {} h per day. Line-by-line detail is in the detailed report for the same month.",
        MAX_DAY_HOURS
    );
    c.text(LEFT, c.y + 23.0, Align::Left, Face::Regular, 9.2, INK2, &note);
    c.y += 23.0;
    signatures(&mut c, 80.0);
    render(c, &format!("Timesheet · Daily summary · {}", month.label()))
}

/// Bold total row framed by thick rules; values are right-aligned at the given x positions.
fn total_row(c: &mut Canvas, label: &str, values: &[(f32, f64)], x_label: f32) {
    let h = 29.0;
    c.ensure(h);
    c.hline(LEFT, RIGHT, c.y, INK, 1.4);
    let base = c.y + 18.5;
    c.text(x_label, base, Align::Left, Face::SemiBold, BODY, INK, label);
    for &(x, v) in values {
        c.text(x, base, Align::Right, Face::MonoSemiBold, BODY, INK, &fmt_report(v));
    }
    c.y += h;
    c.hline(LEFT, RIGHT, c.y, INK, 1.4);
}

fn dash_if_zero(v: f64) -> String {
    if v == 0.0 { "–".into() } else { fmt_report(v) }
}

fn ellipsize(text: &str, face: Face, size: f32, max: f32) -> String {
    if face.width(text, size) <= max {
        return text.to_owned();
    }
    let mut out = String::new();
    for ch in text.chars() {
        out.push(ch);
        if face.width(&format!("{out}…"), size) > max {
            out.pop();
            break;
        }
    }
    format!("{}…", out.trim_end())
}

fn render(canvas: Canvas, title: &str) -> Result<Vec<u8>, String> {
    let fonts: Vec<Font> = Face::ALL
        .iter()
        .map(|f| Font::new(f.bytes().into(), 0).ok_or("embedded font could not be loaded"))
        .collect::<Result<_, _>>()?;
    let font = |face: Face| fonts[face as usize].clone();
    let fill = |c: Rgb| Fill { paint: rgb::Color::new(c.0, c.1, c.2).into(), opacity: NormalizedF32::ONE, rule: FillRule::NonZero };

    let mut document = Document::new();
    document.set_metadata(Metadata::new().title(title.to_owned()));
    for ops in canvas.pages {
        let mut page = document.start_page_with(PageSettings::from_wh(PAGE_W, PAGE_H).expect("A4 size"));
        let mut surface = page.surface();
        for op in ops {
            match op {
                Op::Text { x, y, face, size, color, text } => {
                    surface.set_stroke(None);
                    surface.set_fill(Some(fill(color)));
                    surface.draw_text(Point::from_xy(x, y), font(face), size, &text, false, TextDirection::LeftToRight);
                }
                Op::Rect { x, y, w, h, radius, fill: f, stroke } => {
                    let Some(path) = rect_path(x, y, w, h, radius) else { continue };
                    surface.set_fill(f.map(fill));
                    surface.set_stroke(stroke.map(|(c, width)| Stroke { paint: rgb::Color::new(c.0, c.1, c.2).into(), width, ..Default::default() }));
                    surface.draw_path(&path);
                }
                Op::HLine { x1, x2, y, color, width } => {
                    let Some(path) = Rect::from_xywh(x1, y - width / 2.0, x2 - x1, width).and_then(|r| {
                        let mut pb = PathBuilder::new();
                        pb.push_rect(r);
                        pb.finish()
                    }) else {
                        continue;
                    };
                    surface.set_stroke(None);
                    surface.set_fill(Some(fill(color)));
                    surface.draw_path(&path);
                }
            }
        }
        surface.finish();
        page.finish();
    }
    document.finish().map_err(|e| format!("PDF generation failed: {e:?}"))
}

/// Rectangle path with optional rounded corners; zero width or height gives a straight line.
fn rect_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<krilla::geom::Path> {
    let mut pb = PathBuilder::new();
    if w == 0.0 || h == 0.0 {
        pb.move_to(x, y);
        pb.line_to(x + w, y + h);
        return pb.finish();
    }
    let r = r.min(w / 2.0).min(h / 2.0);
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish()
}

/// Suggested file name, e.g. "Timesheet 2026-10 · Detailed report.pdf".
pub fn file_name(month: Month, report: &str) -> String {
    format!("Timesheet {}-{:02} - {report}.pdf", month.year, month.month)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Entry;

    fn sample() -> Data {
        let oct = Month { year: 2026, month: 10 };
        let mut data = Data::default();
        for (i, d) in oct.weekdays().into_iter().enumerate() {
            let kind = Kind::ALL[i % 3];
            data.entries.push(Entry { id: i as u64 * 2, date: d, kind, desc: format!("Ticket #{} work", 1200 + i), hours: "5".into() });
            data.entries.push(Entry { id: i as u64 * 2 + 1, date: d, kind: Kind::Support, desc: "Support".into(), hours: "3".into() });
        }
        data
    }

    #[test]
    fn reports_render_and_paginate() {
        let oct = Month { year: 2026, month: 10 };
        let data = sample();
        let detailed = detailed_report(&data, oct).unwrap();
        let summary = daily_summary(&data, oct).unwrap();
        assert!(detailed.starts_with(b"%PDF"));
        assert!(summary.starts_with(b"%PDF"));
        // Empty month still renders.
        assert!(detailed_report(&Data::default(), oct).is_ok());
        assert!(daily_summary(&Data::default(), oct).is_ok());
    }

    #[test]
    fn ellipsize_long_descriptions() {
        let long = "x".repeat(400);
        let out = ellipsize(&long, Face::Regular, BODY, 100.0);
        assert!(out.ends_with('…'));
        assert!(Face::Regular.width(&out, BODY) <= 100.0);
        assert_eq!(ellipsize("short", Face::Regular, BODY, 100.0), "short");
    }
}

/// October 2026 data from the reference PDFs; `cargo test -- --ignored` writes both exports to target/.
#[cfg(test)]
mod sample {
    use super::*;
    use crate::model::{Entry, Settings};

    const LINES: &[(u32, Kind, &str, &str)] = &[
        (1, Kind::Backend, "Ticket #1201 payment webhook retries", "8"),
        (2, Kind::Frontend, "Ticket #1210 checkout layout", "5"),
        (2, Kind::Support, "Production incident follow-up", "3"),
        (5, Kind::Backend, "Worked on ticket #1234", "8"),
        (6, Kind::Backend, "Worked on ticket #1234", "4"),
        (6, Kind::Backend, "Worked on ticket #456", "4"),
        (7, Kind::Backend, "Worked on ticket #456", "8"),
        (8, Kind::Frontend, "Checkout form validation", "3"),
        (8, Kind::Support, "Customer call about login issue", "1.5"),
        (8, Kind::Backend, "Ticket #1250 order export endpoint", "3.5"),
        (9, Kind::Frontend, "Ticket #1262 account settings page", "6"),
        (9, Kind::Support, "Release support", "2"),
        (12, Kind::Backend, "Ticket #1270 search indexing", "8"),
        (13, Kind::Backend, "Ticket #1270 search indexing", "5"),
        (13, Kind::Support, "Data fix for customer orders", "3"),
        (14, Kind::Frontend, "Ticket #1281 mobile navigation", "8"),
        (15, Kind::Frontend, "Ticket #1281 mobile navigation", "4"),
        (15, Kind::Frontend, "Ticket #1285 accessibility fixes", "4"),
        (16, Kind::Backend, "Ticket #1290 API rate limiting", "6"),
        (16, Kind::Support, "Weekly support rotation", "2"),
        (19, Kind::Backend, "Ticket #1290 API rate limiting", "8"),
        (20, Kind::Frontend, "Ticket #1302 dashboard charts", "8"),
        (21, Kind::Frontend, "Ticket #1302 dashboard charts", "3"),
        (21, Kind::Backend, "Ticket #1305 reporting queries", "5"),
        (22, Kind::Support, "Weekly support rotation", "4"),
        (22, Kind::Backend, "Ticket #1305 reporting queries", "4"),
        (23, Kind::Backend, "Ticket #1311 invoice PDF generation", "8"),
        (26, Kind::Backend, "Ticket #1311 invoice PDF generation", "4"),
        (26, Kind::Frontend, "Ticket #1315 invoice download UI", "4"),
        (27, Kind::Frontend, "Ticket #1315 invoice download UI", "8"),
        (28, Kind::Support, "Production incident #1320", "3"),
        (28, Kind::Backend, "Ticket #1320 hotfix", "5"),
        (29, Kind::Frontend, "Ticket #1324 onboarding flow", "8"),
        (30, Kind::Frontend, "Ticket #1324 onboarding flow", "6"),
        (30, Kind::Support, "Monthly release support", "2"),
    ];

    pub fn october() -> Data {
        let entries = LINES
            .iter()
            .enumerate()
            .map(|(i, &(day, kind, desc, hours))| Entry {
                id: i as u64,
                date: chrono::NaiveDate::from_ymd_opt(2026, 10, day).unwrap(),
                kind,
                desc: desc.into(),
                hours: hours.into(),
            })
            .collect();
        let settings = Settings { consultant: String::new(), client: String::new(), project: "Website redesign".into() };
        Data { settings, entries, next_id: LINES.len() as u64, ..Default::default() }
    }

    #[test]
    fn october_totals_match_reference() {
        let data = october();
        let oct = Month { year: 2026, month: 10 };
        let totals: Vec<f64> = Kind::ALL.iter().map(|&k| data.kind_total(oct, k)).collect();
        assert_eq!(totals, vec![67.0, 88.5, 20.5]);
        assert_eq!(data.month_total(oct), 176.0);
    }

    #[test]
    #[ignore = "writes sample PDFs for visual comparison"]
    fn write_october_samples() {
        let data = october();
        let oct = Month { year: 2026, month: 10 };
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
        std::fs::write(dir.join("sample-detailed.pdf"), detailed_report(&data, oct).unwrap()).unwrap();
        std::fs::write(dir.join("sample-summary.pdf"), daily_summary(&data, oct).unwrap()).unwrap();
    }
}
