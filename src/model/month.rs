//! Calendar months and weekday labels.

use chrono::{Datelike, Duration, NaiveDate, Weekday};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Data;

    #[test]
    fn october_2026_calendar() {
        let oct = Month {
            year: 2026,
            month: 10,
        };
        assert_eq!(oct.weekdays().len(), 22);
        assert_eq!(Data::default().target_hours(&oct.weekdays()), 176.0);
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
}
