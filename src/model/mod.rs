//! Timesheet data, month calendar and aggregation rules shared by the UI and the PDF exports.

mod month;
mod store;

use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::holidays::Country;

pub use month::{Month, day_range, weekday_long, weekday_short};
pub use store::Store;

/// A day cannot hold more than this many hours.
pub const MAX_DAY_HOURS: f64 = 8.0;

/// Days without a reply after which a sent month needs a follow-up.
pub const FOLLOW_UP_DAYS: i64 = 5;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Frontend,
    Backend,
    Support,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::Frontend, Kind::Backend, Kind::Support];

    pub fn label(self) -> &'static str {
        match self {
            Kind::Frontend => "Frontend",
            Kind::Backend => "Backend",
            Kind::Support => "Support",
        }
    }
}

/// Why a weekday is not worked. A day off counts 0 h toward the target.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum DayOff {
    Holiday,
    Leave,
    Sick,
    Unpaid,
}

impl DayOff {
    pub const ALL: [DayOff; 4] = [DayOff::Holiday, DayOff::Leave, DayOff::Sick, DayOff::Unpaid];

    pub fn label(self) -> &'static str {
        match self {
            DayOff::Holiday => "Public holiday",
            DayOff::Leave => "Leave",
            DayOff::Sick => "Sick leave",
            DayOff::Unpaid => "Unpaid leave",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Entry {
    pub id: u64,
    pub date: NaiveDate,
    pub kind: Kind,
    pub desc: String,
    /// Raw text typed by the user; parsed with [`parse_hours`].
    pub hours: String,
}

impl Entry {
    pub fn hours(&self) -> f64 {
        parse_hours(&self.hours)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Settings {
    pub consultant: String,
    pub client: String,
    pub project: String,
    /// Its public holidays are days off unless marked otherwise.
    #[serde(default)]
    pub country: Option<Country>,
    #[serde(default)]
    pub appearance: Appearance,
}

/// Light or dark colors for the app window. PDFs always use the light palette.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    /// Follows the operating system.
    #[default]
    System,
    Light,
    Dark,
}

impl Appearance {
    pub const ALL: [Appearance; 3] = [Appearance::System, Appearance::Light, Appearance::Dark];

    pub fn label(self) -> &'static str {
        match self {
            Appearance::System => "System",
            Appearance::Light => "Light",
            Appearance::Dark => "Dark",
        }
    }
}

impl Settings {
    pub fn consultant_or_placeholder(&self) -> &str {
        non_empty_or(&self.consultant, "[YOUR NAME]")
    }
    pub fn client_or_placeholder(&self) -> &str {
        non_empty_or(&self.client, "[CLIENT NAME]")
    }
    pub fn project_or_placeholder(&self) -> &str {
        non_empty_or(&self.project, "[PROJECT NAME]")
    }

    /// Up to two initials from the consultant name, e.g. "Jane Doe" -> "JD".
    pub fn initials(&self) -> String {
        self.consultant_or_placeholder()
            .split_whitespace()
            .filter_map(|w| w.chars().find(|c| c.is_alphanumeric()))
            .take(2)
            .flat_map(char::to_uppercase)
            .collect()
    }
}

fn non_empty_or<'a>(value: &'a str, placeholder: &'a str) -> &'a str {
    if value.trim().is_empty() {
        placeholder
    } else {
        value.trim()
    }
}

/// Sending and manager approval of one month's timesheet.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Approval {
    pub sent: Option<NaiveDate>,
    /// Only meaningful while `sent` is set.
    pub approved: Option<NaiveDate>,
    /// Original file name of the signed PDF; the copy lives at [`Store::signed_pdf_path`].
    pub signed_pdf: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ApprovalStatus {
    NotSent,
    Awaiting { days: i64, late: bool },
    Approved,
}

impl Approval {
    pub fn status(&self, today: NaiveDate) -> ApprovalStatus {
        match (self.sent, self.approved) {
            (None, _) => ApprovalStatus::NotSent,
            (Some(_), Some(_)) => ApprovalStatus::Approved,
            (Some(sent), None) => {
                let days = (today - sent).num_days().max(0);
                ApprovalStatus::Awaiting {
                    days,
                    late: days > FOLLOW_UP_DAYS,
                }
            }
        }
    }

    pub fn set_sent(&mut self, sent: bool, today: NaiveDate) {
        if sent {
            self.sent.get_or_insert(today);
        } else {
            self.sent = None;
            self.approved = None;
        }
    }

    pub fn set_approved(&mut self, approved: bool, today: NaiveDate) {
        self.approved = (approved && self.sent.is_some()).then_some(today);
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Data {
    pub settings: Settings,
    pub entries: Vec<Entry>,
    pub next_id: u64,
    /// Keyed by [`Month::key`].
    #[serde(default)]
    pub approvals: BTreeMap<String, Approval>,
    /// Days marked by the user. `None` keeps a public holiday of [`Settings::country`] as a workday.
    #[serde(default)]
    pub days_off: BTreeMap<NaiveDate, Option<DayOff>>,
}

impl Data {
    pub fn lines(&self, date: NaiveDate) -> impl Iterator<Item = &Entry> {
        self.entries.iter().filter(move |e| e.date == date)
    }

    pub fn day_total(&self, date: NaiveDate) -> f64 {
        round_hours(self.lines(date).map(Entry::hours).sum())
    }

    pub fn day_kind_total(&self, date: NaiveDate, kind: Kind) -> f64 {
        round_hours(
            self.lines(date)
                .filter(|e| e.kind == kind)
                .map(Entry::hours)
                .sum(),
        )
    }

    /// Total of several days, e.g. a week.
    pub fn days_total(&self, days: &[NaiveDate]) -> f64 {
        round_hours(days.iter().map(|&d| self.day_total(d)).sum())
    }

    pub fn kind_total(&self, month: Month, kind: Kind) -> f64 {
        round_hours(
            month
                .weekdays()
                .iter()
                .map(|&d| self.day_kind_total(d, kind))
                .sum(),
        )
    }

    pub fn month_total(&self, month: Month) -> f64 {
        self.days_total(&month.weekdays())
    }

    /// The user's mark for `date`, else [`DayOff::Holiday`] for a public holiday.
    /// A day with lines is always a workday.
    pub fn day_off(&self, date: NaiveDate) -> Option<DayOff> {
        if self.lines(date).next().is_some() {
            return None;
        }
        match self.days_off.get(&date) {
            Some(&mark) => mark,
            None => self.holiday_name(date).map(|_| DayOff::Holiday),
        }
    }

    fn holiday_name(&self, date: NaiveDate) -> Option<&'static str> {
        self.settings.country.and_then(|c| c.holiday(date))
    }

    /// "Public holiday · Christmas Day", or the plain label of the mark.
    pub fn day_off_label(&self, date: NaiveDate) -> Option<String> {
        let off = self.day_off(date)?;
        Some(match self.holiday_name(date) {
            Some(name) if off == DayOff::Holiday => format!("{} · {name}", off.label()),
            _ => off.label().to_owned(),
        })
    }

    /// Marks `date` as a day off, or as a workday with `None`.
    pub fn set_day_off(&mut self, date: NaiveDate, off: Option<DayOff>) {
        if off.is_some() || self.holiday_name(date).is_some() {
            self.days_off.insert(date, off);
        } else {
            self.days_off.remove(&date);
        }
    }

    /// The days of `days` that are not days off.
    pub fn workdays(&self, days: &[NaiveDate]) -> Vec<NaiveDate> {
        days.iter()
            .copied()
            .filter(|&d| self.day_off(d).is_none())
            .collect()
    }

    /// Workdays of `days` still under [`MAX_DAY_HOURS`].
    pub fn open_days(&self, days: &[NaiveDate]) -> Vec<NaiveDate> {
        self.workdays(days)
            .into_iter()
            .filter(|&d| self.day_total(d) < MAX_DAY_HOURS)
            .collect()
    }

    /// Hours expected over `days`: [`MAX_DAY_HOURS`] per workday.
    pub fn target_hours(&self, days: &[NaiveDate]) -> f64 {
        self.workdays(days).len() as f64 * MAX_DAY_HOURS
    }

    /// A month with a day over [`MAX_DAY_HOURS`] cannot be exported.
    pub fn has_overfull_day(&self, month: Month) -> bool {
        month
            .weekdays()
            .iter()
            .any(|&d| self.day_total(d) > MAX_DAY_HOURS)
    }

    pub fn approval(&self, month: Month) -> Approval {
        self.approvals
            .get(&month.key())
            .cloned()
            .unwrap_or_default()
    }

    pub fn approval_mut(&mut self, month: Month) -> &mut Approval {
        self.approvals.entry(month.key()).or_default()
    }

    /// Latest date before `date` that has at least one line.
    pub fn previous_filled_day(&self, date: NaiveDate) -> Option<NaiveDate> {
        self.entries
            .iter()
            .map(|e| e.date)
            .filter(|&d| d < date)
            .max()
    }

    /// Task type proposed for a new line: the last one used on this day or the closest earlier day.
    pub fn next_kind(&self, date: NaiveDate) -> Kind {
        if let Some(e) = self.lines(date).last() {
            return e.kind;
        }
        self.previous_filled_day(date)
            .and_then(|d| self.lines(d).last())
            .map_or(Kind::Backend, |e| e.kind)
    }

    fn take_id(&mut self) -> u64 {
        let used = self.entries.iter().map(|e| e.id + 1).max().unwrap_or(0);
        let id = self.next_id.max(used);
        self.next_id = id + 1;
        id
    }

    /// Adds a line pre-filled with the proposed type and the hours left in the day.
    pub fn add_line(&mut self, date: NaiveDate) {
        let left = (MAX_DAY_HOURS - self.day_total(date)).max(0.0);
        let entry = Entry {
            id: self.take_id(),
            date,
            kind: self.next_kind(date),
            desc: String::new(),
            hours: if left > 0.0 {
                fmt_hours(left)
            } else {
                String::new()
            },
        };
        self.entries.push(entry);
    }

    pub fn copy_day(&mut self, from: NaiveDate, to: NaiveDate) {
        let copies: Vec<Entry> = self.lines(from).cloned().collect();
        for mut e in copies {
            e.id = self.take_id();
            e.date = to;
            self.entries.push(e);
        }
    }

    pub fn remove_line(&mut self, id: u64) {
        self.entries.retain(|e| e.id != id);
    }

    /// Every month before `current`, back to the oldest month holding a line, newest first.
    pub fn closed_months(&self, current: Month) -> Vec<Month> {
        let Some(oldest) = self.entries.iter().map(|e| Month::of(e.date)).min() else {
            return Vec::new();
        };
        let mut months = Vec::new();
        let mut m = current.prev();
        while m >= oldest {
            months.push(m);
            m = m.prev();
        }
        months
    }
}

/// Lenient hour parsing: accepts "7,5", treats invalid or negative input as 0 and rounds to hundredths.
pub fn parse_hours(text: &str) -> f64 {
    let value = text.trim().replace(',', ".").parse::<f64>().unwrap_or(0.0);
    if value.is_finite() {
        round_hours(value.max(0.0))
    } else {
        0.0
    }
}

/// Rounds to hundredths of an hour, so sums of typed decimals compare exactly (0.1 + 4.1 + 3.8 == 8.0).
fn round_hours(n: f64) -> f64 {
    // `+ 0.0` turns -0.0 (the sum of an empty iterator) into 0.0.
    (n * 100.0).round() / 100.0 + 0.0
}

/// Compact hours for the UI: "8", "1.5", "0.25".
pub fn fmt_hours(n: f64) -> String {
    round_hours(n).to_string()
}

/// Fixed one-decimal hours for reports: "8.0", "1.5"; keeps a second decimal only when needed.
pub fn fmt_report(n: f64) -> String {
    let rounded = round_hours(n);
    if ((rounded * 10.0).round() - rounded * 10.0).abs() < 1e-9 {
        format!("{rounded:.1}")
    } else {
        format!("{rounded:.2}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
    }

    fn entry(id: u64, day: u32, kind: Kind, hours: &str) -> Entry {
        Entry {
            id,
            date: d(day),
            kind,
            desc: String::new(),
            hours: hours.into(),
        }
    }

    #[test]
    fn hours_parsing_and_format() {
        assert_eq!(parse_hours("7,5"), 7.5);
        assert_eq!(parse_hours(" 3 "), 3.0);
        assert_eq!(parse_hours("abc"), 0.0);
        assert_eq!(parse_hours("-2"), 0.0);
        assert_eq!(parse_hours("NaN"), 0.0);
        assert_eq!(fmt_hours(8.0), "8");
        assert_eq!(fmt_hours(std::iter::empty::<f64>().sum()), "0");
        assert_eq!(fmt_report(-0.0), "0.0");
        assert_eq!(fmt_hours(1.5), "1.5");
        assert_eq!(fmt_report(8.0), "8.0");
        assert_eq!(fmt_report(176.0), "176.0");
        assert_eq!(fmt_report(0.25), "0.25");
    }

    #[test]
    fn add_copy_and_totals() {
        let mut data = Data {
            entries: vec![
                entry(0, 1, Kind::Backend, "5"),
                entry(1, 1, Kind::Support, "1.5"),
            ],
            next_id: 2,
            ..Default::default()
        };
        assert_eq!(data.day_total(d(1)), 6.5);
        data.add_line(d(1));
        let added = data.entries.last().unwrap();
        assert_eq!((added.kind, added.hours.as_str()), (Kind::Support, "1.5"));

        data.add_line(d(2));
        let added = data.entries.last().unwrap();
        assert_eq!((added.kind, added.hours.as_str()), (Kind::Support, "8"));

        data.copy_day(d(1), d(5));
        assert_eq!(data.day_total(d(5)), 8.0);
        let ids: std::collections::HashSet<u64> = data.entries.iter().map(|e| e.id).collect();
        assert_eq!(ids.len(), data.entries.len());

        let oct = Month {
            year: 2026,
            month: 10,
        };
        assert_eq!(
            data.kind_total(oct, Kind::Support),
            1.5 + 1.5 + 8.0 + 1.5 + 1.5
        );
        assert_eq!(data.previous_filled_day(d(5)), Some(d(2)));
        assert_eq!(data.closed_months(oct), vec![]);
        let jan = Month {
            year: 2027,
            month: 1,
        };
        assert_eq!(
            data.closed_months(jan),
            vec![
                Month {
                    year: 2026,
                    month: 12
                },
                Month {
                    year: 2026,
                    month: 11
                },
                oct
            ]
        );
    }

    #[test]
    fn decimal_hours_sum_exactly() {
        let data = Data {
            entries: vec![
                entry(0, 1, Kind::Backend, "0.1"),
                entry(1, 1, Kind::Support, "4.1"),
                entry(2, 1, Kind::Frontend, "3.8"),
            ],
            next_id: 3,
            ..Default::default()
        };
        assert_eq!(data.day_total(d(1)), 8.0);
        assert!(!data.has_overfull_day(Month::of(d(1))));
        assert_eq!(parse_hours("0.125"), 0.13);
    }

    #[test]
    fn marked_day_lowers_the_target_and_the_month_can_be_completed() {
        let oct = Month::of(d(1));
        let mut data = Data::default();
        data.set_day_off(d(5), Some(DayOff::Leave));
        assert_eq!(data.target_hours(&oct.weekdays()), 21.0 * 8.0);
        let week_targets: f64 = oct.weeks().iter().map(|w| data.target_hours(w)).sum();
        assert_eq!(week_targets, data.target_hours(&oct.weekdays()));
        assert_eq!(data.target_hours(&oct.weeks()[1]), 4.0 * 8.0);

        for date in data.workdays(&oct.weekdays()) {
            data.add_line(date);
        }
        assert!(data.open_days(&oct.weekdays()).is_empty());
        assert_eq!(data.month_total(oct), data.target_hours(&oct.weekdays()));
        assert_eq!(data.day_off_label(d(5)).as_deref(), Some("Leave"));

        data.set_day_off(d(5), None);
        assert_eq!(data.day_off(d(5)), None);
        assert!(data.days_off.is_empty());
        assert_eq!(data.open_days(&oct.weekdays()), vec![d(5)]);
    }

    #[test]
    fn public_holidays_of_the_chosen_country_are_days_off() {
        let nov11 = NaiveDate::from_ymd_opt(2026, 11, 11).unwrap();
        let nov = Month::of(nov11);
        let mut data = Data::default();
        assert_eq!(data.day_off(nov11), None);

        data.settings.country = Some(Country::Belgium);
        assert_eq!(data.day_off(nov11), Some(DayOff::Holiday));
        assert_eq!(
            data.day_off_label(nov11).as_deref(),
            Some("Public holiday · Armistice Day")
        );
        // Nov 2026: 21 weekdays, Nov 11 is a Wednesday (Nov 1 is a Sunday).
        assert_eq!(data.target_hours(&nov.weekdays()), 20.0 * 8.0);

        // A holiday can be worked: marking it as a workday is remembered.
        data.set_day_off(nov11, None);
        assert_eq!(data.day_off(nov11), None);
        assert_eq!(data.target_hours(&nov.weekdays()), 21.0 * 8.0);
        data.set_day_off(nov11, Some(DayOff::Sick));
        assert_eq!(data.day_off_label(nov11).as_deref(), Some("Sick leave"));

        // Time entered on a day off makes it a workday.
        data.add_line(nov11);
        assert_eq!(data.day_off(nov11), None);
        let mut data = Data::default();
        data.add_line(nov11);
        data.settings.country = Some(Country::Belgium);
        assert_eq!(data.day_off(nov11), None);
    }

    #[test]
    fn approval_flow() {
        let mut a = Approval::default();
        assert_eq!(a.status(d(8)), ApprovalStatus::NotSent);
        a.set_approved(true, d(8));
        assert_eq!(a.approved, None, "cannot approve before sending");
        a.set_sent(true, d(1));
        assert_eq!(
            a.status(d(6)),
            ApprovalStatus::Awaiting {
                days: 5,
                late: false
            }
        );
        assert_eq!(
            a.status(d(7)),
            ApprovalStatus::Awaiting {
                days: 6,
                late: true
            }
        );
        a.set_sent(true, d(7));
        assert_eq!(a.sent, Some(d(1)), "ticking again keeps the date");
        a.set_approved(true, d(7));
        assert_eq!(a.status(d(8)), ApprovalStatus::Approved);
        a.set_sent(false, d(8));
        assert_eq!((a.sent, a.approved), (None, None));
    }

    #[test]
    fn data_without_approvals_still_loads() {
        let data: Data = serde_json::from_str(
            r#"{"settings":{"consultant":"","client":"","project":""},"entries":[],"next_id":0}"#,
        )
        .unwrap();
        assert!(data.approvals.is_empty());
        assert_eq!(data.settings.appearance, Appearance::System);
    }

    #[test]
    fn initials() {
        let mut s = Settings::default();
        assert_eq!(s.initials(), "YN");
        s.consultant = "jane van doe".into();
        assert_eq!(s.initials(), "JV");
    }
}
