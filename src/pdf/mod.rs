//! The two monthly exports: detailed report (one row per line) and daily summary (one row per weekday).
//!
//! Layout is computed into a list of drawing operations per page, then rendered with krilla.

mod detailed;
mod render;
mod summary;

/// October 2026 data from the reference PDFs; `cargo test -- --ignored` writes both exports to target/.
#[cfg(test)]
mod sample;

use chrono::Datelike;

use crate::fonts;
use crate::model::{Data, Kind, Month, fmt_report};

pub use detailed::detailed_report;
pub use summary::daily_summary;

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

pub const fn kind_rgb(kind: Kind) -> Rgb {
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
    const ALL: [Face; 5] = [
        Face::Regular,
        Face::SemiBold,
        Face::Bold,
        Face::Mono,
        Face::MonoSemiBold,
    ];

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
            .map(|c| {
                face.glyph_index(c)
                    .and_then(|g| face.glyph_hor_advance(g))
                    .unwrap_or(0) as u32
            })
            .sum();
        units as f32 * size / face.units_per_em() as f32
    }
}

enum Op {
    Text {
        x: f32,
        y: f32,
        face: Face,
        size: f32,
        color: Rgb,
        text: String,
    },
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        fill: Option<Rgb>,
        stroke: Option<(Rgb, f32)>,
    },
    HLine {
        x1: f32,
        x2: f32,
        y: f32,
        color: Rgb,
        width: f32,
    },
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
        Canvas {
            pages: vec![Vec::new()],
            y: 0.0,
        }
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

    #[expect(
        clippy::too_many_arguments,
        reason = "drawing primitive: position, style and content"
    )]
    fn text(
        &mut self,
        x: f32,
        baseline: f32,
        align: Align,
        face: Face,
        size: f32,
        color: Rgb,
        text: &str,
    ) {
        let x = match align {
            Align::Left => x,
            Align::Right => x - face.width(text, size),
        };
        self.push(Op::Text {
            x,
            y: baseline,
            face,
            size,
            color,
            text: text.to_owned(),
        });
    }

    /// Text with extra tracking between characters (used for the small uppercase eyebrow).
    #[expect(
        clippy::too_many_arguments,
        reason = "drawing primitive: position, style and content"
    )]
    fn tracked(
        &mut self,
        x: f32,
        baseline: f32,
        face: Face,
        size: f32,
        color: Rgb,
        text: &str,
        tracking: f32,
    ) {
        let mut x = x;
        let mut buf = [0u8; 4];
        for c in text.chars() {
            let s = c.encode_utf8(&mut buf);
            self.text(x, baseline, Align::Left, face, size, color, s);
            x += face.width(s, size) + tracking;
        }
    }

    fn hline(&mut self, x1: f32, x2: f32, y: f32, color: Rgb, width: f32) {
        self.push(Op::HLine {
            x1,
            x2,
            y,
            color,
            width,
        });
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "drawing primitive: position, style and content"
    )]
    fn rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        fill: Option<Rgb>,
        stroke: Option<(Rgb, f32)>,
    ) {
        self.push(Op::Rect {
            x,
            y,
            w,
            h,
            radius,
            fill,
            stroke,
        });
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
    c.text(
        LEFT,
        92.0,
        Align::Left,
        Face::Bold,
        22.5,
        INK,
        &month.label(),
    );
    let subtitle = format!(
        "{} · {}",
        s.project_or_placeholder(),
        s.client_or_placeholder()
    );
    c.text(
        LEFT,
        111.0,
        Align::Left,
        Face::Regular,
        10.5,
        INK2,
        &subtitle,
    );

    let rows = [
        ("Consultant", s.consultant_or_placeholder().to_owned()),
        ("Period", month.period_label()),
        (
            "Workdays",
            data.workdays(&month.weekdays()).len().to_string(),
        ),
    ];
    let value_w = rows
        .iter()
        .map(|(_, v)| Face::SemiBold.width(v, BODY))
        .fold(0.0, f32::max);
    let label_right = RIGHT - value_w.max(52.0) - 12.0;
    for (i, (label, value)) in rows.iter().enumerate() {
        let y = 65.0 + i as f32 * 15.5;
        c.text(
            label_right,
            y,
            Align::Right,
            Face::Regular,
            BODY,
            MUTED,
            label,
        );
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
    let y = if c.at_page_top() {
        c.y + 40.0
    } else {
        c.y + gap
    };
    let column_gap = 31.0;
    let w = (RIGHT - LEFT - column_gap) / 2.0;
    for (i, label) in ["Consultant signature · date", "Client approval · date"]
        .into_iter()
        .enumerate()
    {
        let x = LEFT + i as f32 * (w + column_gap);
        c.hline(x, x + w, y, INK2, 0.8);
        c.text(x, y + 14.0, Align::Left, Face::Regular, BODY, INK2, label);
    }
    c.y = y + 26.0;
}

/// Bold total row framed by thick rules; values are right-aligned at the given x positions.
fn total_row(c: &mut Canvas, label: &str, values: &[(f32, f64)], x_label: f32) {
    let h = 29.0;
    c.ensure(h);
    c.hline(LEFT, RIGHT, c.y, INK, 1.4);
    let base = c.y + 18.5;
    c.text(x_label, base, Align::Left, Face::SemiBold, BODY, INK, label);
    for &(x, v) in values {
        c.text(
            x,
            base,
            Align::Right,
            Face::MonoSemiBold,
            BODY,
            INK,
            &fmt_report(v),
        );
    }
    c.y += h;
    c.hline(LEFT, RIGHT, c.y, INK, 1.4);
}

fn dash_if_zero(v: f64) -> String {
    if v == 0.0 {
        "–".into()
    } else {
        fmt_report(v)
    }
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

/// Suggested file name, e.g. "Timesheet 2026-10 · Detailed report.pdf".
pub fn file_name(month: Month, report: &str) -> String {
    format!("Timesheet {}-{:02} - {report}.pdf", month.year, month.month)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DayOff, Entry};

    fn sample() -> Data {
        let oct = Month {
            year: 2026,
            month: 10,
        };
        let mut data = Data::default();
        for (i, d) in oct.weekdays().into_iter().enumerate() {
            let kind = Kind::ALL[i % 3];
            data.entries.push(Entry {
                id: i as u64 * 2,
                date: d,
                kind,
                desc: format!("Ticket #{} work", 1200 + i),
                hours: "5".into(),
            });
            data.entries.push(Entry {
                id: i as u64 * 2 + 1,
                date: d,
                kind: Kind::Support,
                desc: "Support".into(),
                hours: "3".into(),
            });
        }
        data
    }

    #[test]
    fn reports_render_and_paginate() {
        let oct = Month {
            year: 2026,
            month: 10,
        };
        let mut data = sample();
        data.entries.retain(|e| e.date.day() != 5);
        data.set_day_off(oct.first_day().with_day(5).unwrap(), Some(DayOff::Leave));
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
