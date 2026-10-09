use super::render::render;
use super::*;
use crate::model::{MAX_DAY_HOURS, weekday_short};

/// PDF 2 · Daily summary: one row per weekday with hours per task type.
pub fn daily_summary(data: &Data, month: Month) -> Result<Vec<u8>, String> {
    let mut c = Canvas::new();
    header(&mut c, data, month, "TIMESHEET · DAILY SUMMARY");

    let width = RIGHT - LEFT;
    let x_date = LEFT + 8.0;
    let x_kinds = [
        LEFT + width * 0.43,
        LEFT + width * 0.639,
        LEFT + width * 0.843,
    ];
    // Day-off labels, in place of the task type columns.
    let x_off = LEFT + width * 0.2;
    let total_bg = LEFT + width * 0.858;
    let x_total = RIGHT - 8.0;
    let total_col =
        |c: &mut Canvas, h: f32| c.rect(total_bg, c.y, RIGHT - total_bg, h, 0.0, Some(SOFT), None);

    c.y = 146.0;
    let head_h = 24.6;
    total_col(&mut c, head_h);
    c.hline(LEFT, RIGHT, c.y, INK, 0.8);
    let base = c.y + 15.5;
    c.text(x_date, base, Align::Left, Face::SemiBold, BODY, INK, "Date");
    for (k, x) in Kind::ALL.iter().zip(x_kinds) {
        c.text(x, base, Align::Right, Face::SemiBold, BODY, INK, k.label());
        c.swatch(
            x - Face::SemiBold.width(k.label(), BODY) - 10.0,
            base,
            kind_rgb(*k),
        );
    }
    c.text(
        x_total,
        base,
        Align::Right,
        Face::SemiBold,
        BODY,
        INK,
        "Total",
    );
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
            let label = format!(
                "{}, {} {}",
                weekday_short(date),
                month.short_name(),
                date.day()
            );
            c.text(x_date, base, Align::Left, Face::Regular, BODY, INK, &label);
            if let Some(off) = data.day_off_label(date) {
                // Spans the task type columns, empty on a day off.
                c.text(x_off, base, Align::Left, Face::SemiBold, BODY, INK2, &off);
            } else {
                for (k, x) in Kind::ALL.iter().zip(x_kinds) {
                    c.text(
                        x,
                        base,
                        Align::Right,
                        Face::Mono,
                        BODY,
                        INK,
                        &dash_if_zero(data.day_kind_total(date, *k)),
                    );
                }
            }
            c.text(
                x_total,
                base,
                Align::Right,
                Face::MonoSemiBold,
                BODY,
                INK,
                &dash_if_zero(data.day_total(date)),
            );
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
    c.text(
        LEFT,
        c.y + 23.0,
        Align::Left,
        Face::Regular,
        9.2,
        INK2,
        &note,
    );
    c.y += 23.0;
    signatures(&mut c, 80.0);
    render(c, &format!("Timesheet · Daily summary · {}", month.label()))
}
