//! Public holiday calendars, computed from the date alone so no network access is needed.

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Country {
    Belgium,
}

impl Country {
    pub const ALL: [Country; 1] = [Country::Belgium];

    pub fn label(self) -> &'static str {
        match self {
            Country::Belgium => "Belgium",
        }
    }

    /// Name of the public holiday on `date`, if any.
    pub fn holiday(self, date: NaiveDate) -> Option<&'static str> {
        match self {
            Country::Belgium => belgian_holiday(date),
        }
    }
}

fn belgian_holiday(date: NaiveDate) -> Option<&'static str> {
    let fixed = match (date.month(), date.day()) {
        (1, 1) => Some("New Year's Day"),
        (5, 1) => Some("Labour Day"),
        (7, 21) => Some("National Day"),
        (8, 15) => Some("Assumption Day"),
        (11, 1) => Some("All Saints' Day"),
        (11, 11) => Some("Armistice Day"),
        (12, 25) => Some("Christmas Day"),
        _ => None,
    };
    fixed.or_else(|| match (date - easter_sunday(date.year())).num_days() {
        1 => Some("Easter Monday"),
        39 => Some("Ascension Day"),
        50 => Some("Whit Monday"),
        _ => None,
    })
}

/// Gregorian Easter Sunday (anonymous Gregorian algorithm, Meeus/Jones/Butcher).
fn easter_sunday(year: i32) -> NaiveDate {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = (h + l - 7 * m + 114) % 31 + 1;
    NaiveDate::from_ymd_opt(year, month as u32, day as u32).expect("Easter is a valid date")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn easter_sunday_known_years() {
        assert_eq!(easter_sunday(2024), d(2024, 3, 31));
        assert_eq!(easter_sunday(2025), d(2025, 4, 20));
        assert_eq!(easter_sunday(2026), d(2026, 4, 5));
        assert_eq!(easter_sunday(2038), d(2038, 4, 25));
    }

    #[test]
    fn belgian_holidays_2026() {
        let be = Country::Belgium;
        let holidays: Vec<(NaiveDate, &str)> = d(2026, 1, 1)
            .iter_days()
            .take_while(|date| *date < d(2027, 1, 1))
            .filter_map(|date| be.holiday(date).map(|name| (date, name)))
            .collect();
        assert_eq!(
            holidays,
            vec![
                (d(2026, 1, 1), "New Year's Day"),
                (d(2026, 4, 6), "Easter Monday"),
                (d(2026, 5, 1), "Labour Day"),
                (d(2026, 5, 14), "Ascension Day"),
                (d(2026, 5, 25), "Whit Monday"),
                (d(2026, 7, 21), "National Day"),
                (d(2026, 8, 15), "Assumption Day"),
                (d(2026, 11, 1), "All Saints' Day"),
                (d(2026, 11, 11), "Armistice Day"),
                (d(2026, 12, 25), "Christmas Day"),
            ]
        );
    }
}
