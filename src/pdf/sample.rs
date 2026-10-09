use super::*;
use crate::model::{Entry, Settings};

const LINES: &[(u32, Kind, &str, &str)] = &[
    (
        1,
        Kind::Backend,
        "Ticket #1201 payment webhook retries",
        "8",
    ),
    (2, Kind::Frontend, "Ticket #1210 checkout layout", "5"),
    (2, Kind::Support, "Production incident follow-up", "3"),
    (5, Kind::Backend, "Worked on ticket #1234", "8"),
    (6, Kind::Backend, "Worked on ticket #1234", "4"),
    (6, Kind::Backend, "Worked on ticket #456", "4"),
    (7, Kind::Backend, "Worked on ticket #456", "8"),
    (8, Kind::Frontend, "Checkout form validation", "3"),
    (8, Kind::Support, "Customer call about login issue", "1.5"),
    (
        8,
        Kind::Backend,
        "Ticket #1250 order export endpoint",
        "3.5",
    ),
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
    (
        23,
        Kind::Backend,
        "Ticket #1311 invoice PDF generation",
        "8",
    ),
    (
        26,
        Kind::Backend,
        "Ticket #1311 invoice PDF generation",
        "4",
    ),
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
    let settings = Settings {
        consultant: String::new(),
        client: String::new(),
        project: "Website redesign".into(),
        ..Settings::default()
    };
    Data {
        settings,
        entries,
        next_id: LINES.len() as u64,
        ..Default::default()
    }
}

#[test]
fn october_totals_match_reference() {
    let data = october();
    let oct = Month {
        year: 2026,
        month: 10,
    };
    let totals: Vec<f64> = Kind::ALL.iter().map(|&k| data.kind_total(oct, k)).collect();
    assert_eq!(totals, vec![67.0, 88.5, 20.5]);
    assert_eq!(data.month_total(oct), 176.0);
}

#[test]
#[ignore = "writes sample PDFs for visual comparison"]
fn write_october_samples() {
    let data = october();
    let oct = Month {
        year: 2026,
        month: 10,
    };
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    std::fs::write(
        dir.join("sample-detailed.pdf"),
        detailed_report(&data, oct).unwrap(),
    )
    .unwrap();
    std::fs::write(
        dir.join("sample-summary.pdf"),
        daily_summary(&data, oct).unwrap(),
    )
    .unwrap();
}
