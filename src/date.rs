//! Calendar helpers (no external date crate needed).
//!
//! Uses Howard Hinnant's `days_from_civil` / `civil_from_days` algorithms and
//! the local Windows clock, so the date format is the same as the one the
//! AutoIt version wrote (`YYYY/MM/DD`).

#[cfg(windows)]
use windows_sys::Win32::Foundation::SYSTEMTIME;
#[cfg(windows)]
use windows_sys::Win32::System::SystemInformation::GetLocalTime;

/// Today's local date as `(year, month, day)`.
#[cfg(windows)]
pub fn today() -> (i32, u32, u32) {
    unsafe {
        let mut system_time: SYSTEMTIME = std::mem::zeroed();
        GetLocalTime(&mut system_time);
        (
            system_time.wYear as i32,
            system_time.wMonth as u32,
            system_time.wDay as u32,
        )
    }
}

/// Fallback used when the crate is built off Windows, only so the date maths
/// below can be unit tested on any platform.
#[cfg(not(windows))]
pub fn today() -> (i32, u32, u32) {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0);
    civil_from_days(seconds.div_euclid(86_400))
}

/// Adds (or subtracts, with a negative value) days to a date.
pub fn add_days(date: (i32, u32, u32), days: i64) -> (i32, u32, u32) {
    civil_from_days(days_from_civil(date.0 as i64, date.1 as i64, date.2 as i64) + days)
}

/// Whole days from `from` to `to` (negative when `to` lies in the past).
pub fn days_between(from: (i32, u32, u32), to: (i32, u32, u32)) -> i64 {
    days_from_civil(to.0 as i64, to.1 as i64, to.2 as i64)
        - days_from_civil(from.0 as i64, from.1 as i64, from.2 as i64)
}

/// Formats a date as `YYYY/MM/DD`.
pub fn format(date: (i32, u32, u32)) -> String {
    format!("{:04}/{:02}/{:02}", date.0, date.1, date.2)
}

/// Parses `YYYY/MM/DD` and rejects impossible dates such as `2026/02/31`.
pub fn parse(text: &str) -> Option<(i32, u32, u32)> {
    let mut parts = text.trim().split('/');
    let year: i32 = parts.next()?.trim().parse().ok()?;
    let month: u32 = parts.next()?.trim().parse().ok()?;
    let day: u32 = parts.next()?.trim().parse().ok()?;
    if parts.next().is_some() || !(1970..=9999).contains(&year) {
        return None;
    }

    let round_trip = civil_from_days(days_from_civil(year as i64, month as i64, day as i64));
    match round_trip {
        (y, m, d) if (y, m, d) == (year, month, day) => Some((year, month, day)),
        _ => None,
    }
}

/// Days since 1970-01-01 for a proleptic Gregorian date.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400; // [0, 399]
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146097 + day_of_era - 719468
}

/// Inverse of [`days_from_civil`].
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let days = days + 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let day_of_era = days - era * 146097; // [0, 146096]
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365; // [0, 399]
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100); // [0, 365]
    let month_prime = (5 * day_of_year + 2) / 153; // [0, 11]
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1; // [1, 31]
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    }; // [1, 12]

    (
        (year + i64::from(month <= 2)) as i32,
        month as u32,
        day as u32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_and_leap_days() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 1, 1), 10_957);
        assert_eq!(days_from_civil(2026, 1, 1), 20_454);
        // 2024 was a leap year: the day after Feb 28 was Feb 29.
        assert_eq!(
            days_from_civil(2024, 3, 1) - days_from_civil(2024, 2, 28),
            2
        );
        assert_eq!(
            days_from_civil(2026, 3, 1) - days_from_civil(2026, 2, 28),
            1
        );
    }

    #[test]
    fn round_trip_over_a_long_range() {
        for days in (0..200_000).step_by(7) {
            let date = civil_from_days(days);
            assert_eq!(
                days_from_civil(date.0 as i64, date.1 as i64, date.2 as i64),
                days
            );
        }
    }

    #[test]
    fn adding_days_crosses_months_and_years() {
        assert_eq!(add_days((2026, 1, 1), 15), (2026, 1, 16));
        assert_eq!(add_days((2026, 1, 31), 1), (2026, 2, 1));
        assert_eq!(add_days((2026, 12, 31), 1), (2027, 1, 1));
        assert_eq!(add_days((2024, 2, 28), 1), (2024, 2, 29));
        assert_eq!(add_days((2026, 10, 19), -19), (2026, 9, 30));
    }

    #[test]
    fn days_between_counts_whole_days() {
        assert_eq!(days_between((2026, 10, 19), (2026, 11, 3)), 15);
        assert_eq!(days_between((2026, 11, 3), (2026, 10, 19)), -15);
        assert_eq!(days_between((2026, 10, 19), (2026, 10, 19)), 0);
    }

    #[test]
    fn formats_the_way_the_reg_file_does() {
        assert_eq!(format((2026, 10, 3)), "2026/10/03");
        assert_eq!(format((2026, 1, 5)), "2026/01/05");
    }

    #[test]
    fn parses_valid_dates_only() {
        assert_eq!(parse("2026/11/03"), Some((2026, 11, 3)));
        assert_eq!(parse(" 2026/11/03 "), Some((2026, 11, 3)));
        assert_eq!(parse("2026/11/03/04"), None);
        assert_eq!(parse("2026/02/31"), None);
        assert_eq!(parse("2026/13/01"), None);
        assert_eq!(parse("2026/00/01"), None);
        assert_eq!(parse("1969/12/31"), None);
        assert_eq!(parse("not a date"), None);
        assert_eq!(parse(""), None);
    }

    #[test]
    fn today_is_a_plausible_date() {
        let (year, month, day) = today();
        assert!((2020..=2100).contains(&year));
        assert!((1..=12).contains(&month));
        assert!((1..=31).contains(&day));
        assert!(days_between((2026, 1, 1), (year, month, day)) > 0);
    }
}
