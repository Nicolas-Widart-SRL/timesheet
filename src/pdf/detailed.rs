use super::render::render;
use super::*;
use crate::model::{day_range, weekday_short};

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
            c.push(Op::Rect {
                x,
                y: top,
                w: 0.0,
                h,
                radius: 0.0,
                fill: None,
                stroke: Some((LINE, 0.7)),
            });
        }
        c.swatch(x + 11.0, top + 17.5, color);
        c.text(
            x + 19.5,
            top + 17.5,
            Align::Left,
            Face::Regular,
            BODY,
            INK2,
            label,
        );
        c.text(
            x + 11.0,
            top + 38.5,
            Align::Left,
            Face::MonoSemiBold,
            14.5,
            INK,
            &format!("{} h", fmt_report(hours)),
        );
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
    c.text(
        x_desc,
        base,
        Align::Left,
        Face::SemiBold,
        BODY,
        INK,
        "Description",
    );
    c.text(
        x_hours,
        base,
        Align::Right,
        Face::SemiBold,
        BODY,
        INK,
        "Hours",
    );
    c.text(x_day, base, Align::Right, Face::SemiBold, BODY, INK, "Day");
    c.y += 24.0;
    c.hline(LEFT, RIGHT, c.y, INK, 0.8);

    for week in month.weeks() {
        let week_total = data.days_total(&week);
        let first_lines = data.lines(week[0]).count().max(1);
        c.ensure(ROW_H * (1 + first_lines) as f32);
        let base = c.y + 14.0;
        let label = format!("{} {}", month.short_name(), day_range(&week, " – "));
        c.text(x_date, base, Align::Left, Face::SemiBold, BODY, INK, &label);
        c.text(
            x_day,
            base,
            Align::Right,
            Face::MonoSemiBold,
            BODY,
            INK,
            &fmt_report(week_total),
        );
        c.y += ROW_H;
        c.hline(LEFT, RIGHT, c.y, INK, 0.8);

        for (i, &date) in week.iter().enumerate() {
            let off = data.day_off_label(date);
            let lines: Vec<_> = data.lines(date).collect();
            c.ensure(ROW_H * lines.len().max(1) as f32);
            if i > 0 && !c.at_page_top() {
                c.hline(LEFT, RIGHT, c.y, LINE, 0.6);
            }
            let base = c.y + 14.0;
            c.text(
                x_date,
                base,
                Align::Left,
                Face::SemiBold,
                BODY,
                INK,
                &format!("{} {}", weekday_short(date), date.day()),
            );
            c.text(
                x_day,
                base,
                Align::Right,
                Face::MonoSemiBold,
                BODY,
                INK,
                &fmt_report(data.day_total(date)),
            );
            if lines.is_empty() {
                let (face, color, note) = match &off {
                    Some(label) => (Face::SemiBold, INK2, label.as_str()),
                    None => (Face::Regular, MUTED, "No time recorded"),
                };
                c.text(x_desc, base, Align::Left, face, BODY, color, note);
                c.y += ROW_H;
            }
            for line in lines {
                // Only breaks the page when a single day has more lines than a page holds.
                c.ensure(ROW_H);
                let base = c.y + 14.0;
                c.swatch(x_type, base, kind_rgb(line.kind));
                c.text(
                    x_type + 10.5,
                    base,
                    Align::Left,
                    Face::Regular,
                    BODY,
                    INK,
                    line.kind.label(),
                );
                c.text(
                    x_desc,
                    base,
                    Align::Left,
                    Face::Regular,
                    BODY,
                    INK,
                    &ellipsize(&line.desc, Face::Regular, BODY, desc_max),
                );
                c.text(
                    x_hours,
                    base,
                    Align::Right,
                    Face::Mono,
                    BODY,
                    INK,
                    &fmt_report(line.hours()),
                );
                c.y += ROW_H;
            }
        }
    }

    total_row(
        &mut c,
        &format!("Total {}", month.label()),
        &[(x_day, data.month_total(month))],
        x_date,
    );
    signatures(&mut c, 71.0);
    render(
        c,
        &format!("Timesheet · Detailed report · {}", month.label()),
    )
}
