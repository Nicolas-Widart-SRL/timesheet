//! Timesheet data, month calendar and aggregation rules shared by the UI and the PDF exports.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Month {
    pub year: i32,
    pub month: u32,
}

const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

impl Month {
    pub fn of(date: NaiveDate) -> Self {
        Month {
            year: date.year(),
            month: date.month(),
        }
    }

    pub fn first_day(self) -> NaiveDate {
        NaiveDate::from_ymd_opt(self.year, self.month, 1).expect("valid month")
    }

    pub fn next(self) -> Self {
        if self.month == 12 {
            Month {
                year: self.year + 1,
                month: 1,
            }
        } else {
            Month {
                month: self.month + 1,
                ..self
            }
        }
    }

    pub fn prev(self) -> Self {
        if self.month == 1 {
            Month {
                year: self.year - 1,
                month: 12,
            }
        } else {
            Month {
                month: self.month - 1,
                ..self
            }
        }
    }

    pub fn name(self) -> &'static str {
        MONTH_NAMES[self.month as usize - 1]
    }

    pub fn short_name(self) -> &'static str {
        &self.name()[..3]
    }

    /// "2026-10"
    pub fn key(self) -> String {
        format!("{}-{:02}", self.year, self.month)
    }

    /// Inverse of [`Month::key`].
    pub fn from_key(key: &str) -> Option<Self> {
        let (year, month) = key.split_once('-')?;
        let month = Month {
            year: year.parse().ok()?,
            month: month.parse().ok()?,
        };
        ((1..=12).contains(&month.month) && month.key() == key).then_some(month)
    }

    /// "October 2026"
    pub fn label(self) -> String {
        format!("{} {}", self.name(), self.year)
    }

    /// Monday to Friday dates of the month.
    pub fn weekdays(self) -> Vec<NaiveDate> {
        let next_first = self.next().first_day();
        let mut days = Vec::new();
        let mut d = self.first_day();
        while d < next_first {
            if !matches!(d.weekday(), Weekday::Sat | Weekday::Sun) {
                days.push(d);
            }
            d += Duration::days(1);
        }
        days
    }

    /// Weekdays grouped into calendar weeks (a new week starts on Monday).
    pub fn weeks(self) -> Vec<Vec<NaiveDate>> {
        let mut weeks: Vec<Vec<NaiveDate>> = Vec::new();
        for d in self.weekdays() {
            match weeks.last_mut() {
                Some(week) if d.weekday() != Weekday::Mon => week.push(d),
                _ => weeks.push(vec![d]),
            }
        }
        weeks
    }

    pub fn target_hours(self) -> f64 {
        self.weekdays().len() as f64 * MAX_DAY_HOURS
    }

    /// "Oct 1 – 30, 2026": first to last weekday.
    pub fn period_label(self) -> String {
        let days = self.weekdays();
        format!(
            "{} {}, {}",
            self.short_name(),
            day_range(&days, " – "),
            self.year
        )
    }
}

/// "1 – 2" for a range, "30" for a single day.
pub fn day_range(days: &[NaiveDate], separator: &str) -> String {
    match (days.first(), days.last()) {
        (Some(a), Some(b)) if a != b => format!("{}{separator}{}", a.day(), b.day()),
        (Some(a), _) => a.day().to_string(),
        _ => String::new(),
    }
}

pub fn weekday_short(date: NaiveDate) -> &'static str {
    match date.weekday() {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    }
}

pub fn weekday_long(date: NaiveDate) -> &'static str {
    match date.weekday() {
        Weekday::Mon => "Monday",
        Weekday::Tue => "Tuesday",
        Weekday::Wed => "Wednesday",
        Weekday::Thu => "Thursday",
        Weekday::Fri => "Friday",
        Weekday::Sat => "Saturday",
        Weekday::Sun => "Sunday",
    }
}

/// Portable copy of everything the app keeps: the data file fields plus the signed PDFs,
/// base64-encoded and keyed by [`Month::key`]. A plain data file is a valid backup without signed PDFs.
#[derive(Serialize, Deserialize)]
struct Backup {
    #[serde(flatten)]
    data: Data,
    #[serde(default)]
    signed_pdfs: BTreeMap<String, String>,
}

/// JSON file holding all data, written atomically after every change.
pub struct Store {
    path: PathBuf,
    pub data: Data,
    /// Set when an existing file could not be read nor moved aside: saving would overwrite it.
    save_blocked: bool,
}

impl Store {
    /// Loads the store. An unparsable file is moved aside (never overwritten) and a warning is returned.
    /// A file that can be neither read nor moved aside is left untouched: saving is blocked for this session.
    pub fn open(path: PathBuf) -> (Self, Option<String>) {
        let mut warning = None;
        let mut save_blocked = false;
        let data = match fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice(&bytes) {
                Ok(data) => data,
                Err(err) => {
                    let backup = path.with_extension(format!(
                        "corrupt-{}.json",
                        chrono::Local::now().format("%Y%m%d-%H%M%S")
                    ));
                    warning = Some(match fs::rename(&path, &backup) {
                        Ok(()) => format!(
                            "Saved data could not be read ({err}). It was moved to {}.",
                            backup.display()
                        ),
                        Err(e) => {
                            save_blocked = true;
                            format!(
                                "Saved data could not be read ({err}) nor moved aside ({e}). Changes will not be saved."
                            )
                        }
                    });
                    Data::default()
                }
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => Data::default(),
            Err(e) => {
                save_blocked = true;
                warning = Some(format!(
                    "Could not read {}: {e}. Changes will not be saved.",
                    path.display()
                ));
                Data::default()
            }
        };
        (
            Store {
                path,
                data,
                save_blocked,
            },
            warning,
        )
    }

    /// Where the signed PDF of `month` is kept, next to the data file.
    pub fn signed_pdf_path(&self, month: Month) -> PathBuf {
        self.path
            .with_file_name("signed")
            .join(format!("{}.pdf", month.key()))
    }

    /// Copies `source` into the data folder as the signed PDF of `month`.
    /// A signed copy means the month was sent and approved, so missing dates are set to `today`.
    pub fn attach_signed_pdf(
        &mut self,
        month: Month,
        source: &Path,
        today: NaiveDate,
    ) -> io::Result<()> {
        let target = self.signed_pdf_path(month);
        if let Some(dir) = target.parent() {
            fs::create_dir_all(dir)?;
        }
        write_atomic(&target, &fs::read(source)?)?;
        let approval = self.data.approval_mut(month);
        approval.signed_pdf = Some(
            source
                .file_name()
                .map_or_else(|| "signed.pdf".into(), |n| n.to_string_lossy().into_owned()),
        );
        approval.set_sent(true, today);
        approval.approved.get_or_insert(today);
        Ok(())
    }

    pub fn remove_signed_pdf(&mut self, month: Month) -> io::Result<()> {
        match fs::remove_file(self.signed_pdf_path(month)) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
            _ => {}
        }
        self.data.approval_mut(month).signed_pdf = None;
        Ok(())
    }

    /// All data and signed PDFs as one JSON file. A signed copy missing on disk is left out.
    pub fn export_backup(&self) -> io::Result<Vec<u8>> {
        let mut signed_pdfs = BTreeMap::new();
        for (key, approval) in &self.data.approvals {
            let Some(month) = Month::from_key(key).filter(|_| approval.signed_pdf.is_some()) else {
                continue;
            };
            match fs::read(self.signed_pdf_path(month)) {
                Ok(bytes) => {
                    signed_pdfs.insert(key.clone(), BASE64.encode(bytes));
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
        }
        serde_json::to_vec_pretty(&Backup {
            data: self.data.clone(),
            signed_pdfs,
        })
        .map_err(io::Error::other)
    }

    /// Replaces all data and signed PDFs with a backup from [`Store::export_backup`].
    /// The backup is fully checked before anything is written. Unless the store is empty, the current
    /// data is first written to a backup next to the data file, whose path is returned.
    pub fn import_backup(
        &mut self,
        bytes: &[u8],
        now: chrono::NaiveDateTime,
    ) -> io::Result<Option<PathBuf>> {
        let invalid = |msg: String| io::Error::new(io::ErrorKind::InvalidData, msg);
        if self.save_blocked {
            return Err(io::Error::other(
                "the existing data file could not be read, so it is left untouched",
            ));
        }
        let backup: Backup = serde_json::from_slice(bytes)
            .map_err(|e| invalid(format!("not a timesheet backup ({e})")))?;
        let mut ids = HashSet::new();
        if let Some(e) = backup.data.entries.iter().find(|e| !ids.insert(e.id)) {
            return Err(invalid(format!("two lines share the id {}", e.id)));
        }
        let mut pdfs = Vec::new();
        for (key, encoded) in &backup.signed_pdfs {
            let month =
                Month::from_key(key).ok_or_else(|| invalid(format!("invalid month \"{key}\"")))?;
            let pdf = BASE64
                .decode(encoded)
                .map_err(|e| invalid(format!("signed PDF of {key} is damaged ({e})")))?;
            pdfs.push((month, pdf));
        }

        let previous = if self.data == Data::default() {
            None
        } else {
            let previous = self.path.with_extension(format!(
                "before-import-{}.json",
                now.format("%Y%m%d-%H%M%S")
            ));
            if let Some(dir) = previous.parent() {
                fs::create_dir_all(dir)?;
            }
            write_atomic(&previous, &self.export_backup()?)?;
            Some(previous)
        };
        for (month, pdf) in &pdfs {
            let target = self.signed_pdf_path(*month);
            if let Some(dir) = target.parent() {
                fs::create_dir_all(dir)?;
            }
            write_atomic(&target, pdf)?;
        }
        let mut data = backup.data;
        for (key, approval) in &mut data.approvals {
            if Month::from_key(key).is_none_or(|m| !self.signed_pdf_path(m).exists()) {
                approval.signed_pdf = None;
            }
        }
        self.data = data;
        self.save()?;
        Ok(previous)
    }

    pub fn save(&self) -> io::Result<()> {
        if self.save_blocked {
            return Err(io::Error::other(
                "the existing data file could not be read, so it is left untouched",
            ));
        }
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_vec_pretty(&self.data).map_err(io::Error::other)?;
        write_atomic(&self.path, &json)
    }
}

/// Writes to a sibling temp file, flushes it to disk, then renames it over the target
/// so a crash never leaves a half-written file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(&tmp, path)
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
    fn october_2026_calendar() {
        let oct = Month {
            year: 2026,
            month: 10,
        };
        assert_eq!(oct.weekdays().len(), 22);
        assert_eq!(oct.target_hours(), 176.0);
        assert_eq!(oct.period_label(), "Oct 1 – 30, 2026");
        let weeks = oct.weeks();
        assert_eq!(weeks.len(), 5);
        assert_eq!(day_range(&weeks[0], "–"), "1–2");
        assert_eq!(day_range(&weeks[4], " – "), "26 – 30");
        assert_eq!(
            oct.next(),
            Month {
                year: 2026,
                month: 11
            }
        );
        assert_eq!(
            Month {
                year: 2026,
                month: 1
            }
            .prev(),
            Month {
                year: 2025,
                month: 12
            }
        );
    }

    #[test]
    fn month_starting_on_weekend() {
        let nov = Month {
            year: 2026,
            month: 11,
        };
        assert_eq!(
            nov.weekdays()[0],
            NaiveDate::from_ymd_opt(2026, 11, 2).unwrap()
        );
        assert_eq!(nov.period_label(), "Nov 2 – 30, 2026");
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
    fn signed_pdf_is_copied_and_removed() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("my-signed.pdf");
        fs::write(&source, b"%PDF").unwrap();
        let (mut store, _) = Store::open(dir.path().join("data").join("timesheet.json"));
        let sep = Month {
            year: 2026,
            month: 9,
        };
        store.attach_signed_pdf(sep, &source, d(8)).unwrap();
        let copy = store.signed_pdf_path(sep);
        assert_eq!(fs::read(&copy).unwrap(), b"%PDF");
        let a = store.data.approval(sep);
        assert_eq!(
            (a.sent, a.approved, a.signed_pdf.as_deref()),
            (Some(d(8)), Some(d(8)), Some("my-signed.pdf"))
        );
        store.remove_signed_pdf(sep).unwrap();
        assert!(!copy.exists() && source.exists());
        assert_eq!(store.data.approval(sep).signed_pdf, None);
    }

    #[test]
    fn data_without_approvals_still_loads() {
        let data: Data = serde_json::from_str(
            r#"{"settings":{"consultant":"","client":"","project":""},"entries":[],"next_id":0}"#,
        )
        .unwrap();
        assert!(data.approvals.is_empty());
    }

    #[test]
    fn initials() {
        let mut s = Settings::default();
        assert_eq!(s.initials(), "YN");
        s.consultant = "jane van doe".into();
        assert_eq!(s.initials(), "JV");
    }

    #[test]
    fn store_roundtrip_and_corrupt_file_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("timesheet.json");
        let (mut store, warning) = Store::open(path.clone());
        assert!(warning.is_none());
        store.data.add_line(d(1));
        store.save().unwrap();
        assert_eq!(Store::open(path.clone()).0.data, store.data);

        fs::write(&path, b"{not json").unwrap();
        let (store, warning) = Store::open(path.clone());
        assert!(warning.is_some());
        assert!(store.data.entries.is_empty());
        let moved = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .any(|e| e.file_name().to_string_lossy().contains("corrupt"));
        assert!(moved);
    }

    #[test]
    fn month_key_roundtrip() {
        let oct = Month {
            year: 2026,
            month: 10,
        };
        assert_eq!(Month::from_key(&oct.key()), Some(oct));
        for bad in ["2026-13", "2026-1", "2026-00", "../x", "2026", ""] {
            assert_eq!(Month::from_key(bad), None, "{bad}");
        }
    }

    #[test]
    fn backup_moves_data_and_signed_pdfs_to_another_store() {
        let now = d(9).and_hms_opt(10, 0, 0).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("signed.pdf");
        fs::write(&source, b"%PDF-signed").unwrap();
        let (mut from, _) = Store::open(dir.path().join("a").join("timesheet.json"));
        from.data.settings.consultant = "Jane Doe".into();
        from.data.add_line(d(1));
        let sep = Month {
            year: 2026,
            month: 9,
        };
        from.attach_signed_pdf(sep, &source, d(8)).unwrap();
        let backup = from.export_backup().unwrap();

        let (mut to, _) = Store::open(dir.path().join("b").join("timesheet.json"));
        assert_eq!(
            to.import_backup(&backup, now).unwrap(),
            None,
            "an empty store needs no backup"
        );
        assert_eq!(to.data, from.data);
        assert_eq!(fs::read(to.signed_pdf_path(sep)).unwrap(), b"%PDF-signed");
        assert_eq!(
            Store::open(dir.path().join("b").join("timesheet.json"))
                .0
                .data,
            from.data,
            "import is saved"
        );

        // Importing again keeps the replaced data in a backup that can itself be imported.
        to.data.add_line(d(2));
        let replaced = to.data.clone();
        let previous = to
            .import_backup(&backup, now)
            .unwrap()
            .expect("current data is backed up");
        assert_eq!(to.data, from.data);
        to.import_backup(&fs::read(previous).unwrap(), now).unwrap();
        assert_eq!(to.data, replaced);
    }

    #[test]
    fn plain_data_file_imports_without_dangling_signed_pdf() {
        let dir = tempfile::tempdir().unwrap();
        let mut data = Data::default();
        data.add_line(d(1));
        data.approval_mut(Month {
            year: 2026,
            month: 9,
        })
        .signed_pdf = Some("lost.pdf".into());
        let (mut store, _) = Store::open(dir.path().join("timesheet.json"));
        store
            .import_backup(&serde_json::to_vec(&data).unwrap(), d(9).into())
            .unwrap();
        assert_eq!(store.data.entries, data.entries);
        assert_eq!(
            store
                .data
                .approval(Month {
                    year: 2026,
                    month: 9
                })
                .signed_pdf,
            None
        );
    }

    #[test]
    fn invalid_backup_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("timesheet.json");
        let (mut store, _) = Store::open(path.clone());
        store.data.add_line(d(1));
        store.save().unwrap();
        let before = store.data.clone();
        let line = r#"{"id":0,"date":"2026-10-01","kind":"backend","desc":"","hours":"8"}"#;
        let settings = r#""settings":{"consultant":"","client":"","project":""}"#;
        for bad in [
            "{not json".to_owned(),
            format!(r#"{{{settings},"entries":[{line},{line}],"next_id":1}}"#),
            format!(r#"{{{settings},"entries":[],"next_id":0,"signed_pdfs":{{"../evil":""}}}}"#),
            format!(r#"{{{settings},"entries":[],"next_id":0,"signed_pdfs":{{"2026-09":"%%%"}}}}"#),
        ] {
            assert!(
                store.import_backup(bad.as_bytes(), d(9).into()).is_err(),
                "{bad}"
            );
            assert_eq!(store.data, before);
        }
        assert_eq!(
            fs::read_dir(dir.path()).unwrap().count(),
            1,
            "nothing is written"
        );
    }

    #[test]
    fn unreadable_file_is_never_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        // A directory in place of the file cannot be read.
        let path = dir.path().join("timesheet.json");
        fs::create_dir(&path).unwrap();
        let (mut store, warning) = Store::open(path.clone());
        assert!(warning.is_some());
        store.data.add_line(d(1));
        assert!(store.save().is_err());
        assert!(path.is_dir());
        assert!(
            !dir.path().join("timesheet.tmp").exists(),
            "nothing is written"
        );
    }
}
