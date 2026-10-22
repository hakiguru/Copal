//! Dates without a date library: UTC timestamps for file headers and the print history.

use std::time::{SystemTime, UNIX_EPOCH};

/// Calendar date and time in UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DateTime {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub millis: u32,
}

impl DateTime {
    pub fn now() -> Self {
        let d = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        Self::from_unix_millis(d.as_millis() as i64)
    }

    pub fn from_unix_millis(ms: i64) -> Self {
        let secs = ms.div_euclid(1000);
        let millis = ms.rem_euclid(1000) as u32;
        let days = secs.div_euclid(86_400);
        let rem = secs.rem_euclid(86_400);
        // Civil-from-days (Howard Hinnant's algorithm).
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let year = (yoe + era * 400 + if month <= 2 { 1 } else { 0 }) as i32;
        Self {
            year,
            month,
            day,
            hour: (rem / 3600) as u32,
            minute: (rem % 3600 / 60) as u32,
            second: (rem % 60) as u32,
            millis,
        }
    }

    /// `2026-10-22T09:05:42.955Z`
    pub fn iso(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
            self.year, self.month, self.day, self.hour, self.minute, self.second, self.millis
        )
    }

    /// `2026-10-22_09-05-42`, safe in file names and sorts by time.
    pub fn file_stamp(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}_{:02}-{:02}-{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }

    /// `2026-10-22 09:05` (UTC)
    pub fn short(&self) -> String {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute
        )
    }
}

/// `3 h 07 min`, `12 min`, `45 s`
pub fn format_duration(seconds: f64) -> String {
    let s = seconds.max(0.0).round() as u64;
    if s >= 3600 {
        format!("{} h {:02} min", s / 3600, s % 3600 / 60)
    } else if s >= 60 {
        format!("{} min", s / 60)
    } else {
        format!("{s} s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_dates() {
        assert_eq!(
            DateTime::from_unix_millis(0).iso(),
            "1970-01-01T00:00:00.000Z"
        );
        assert_eq!(
            DateTime::from_unix_millis(1_792_659_942_955).iso(),
            "2026-10-22T09:05:42.955Z"
        );
        assert_eq!(
            DateTime::from_unix_millis(951_782_400_000).iso(),
            "2000-02-29T00:00:00.000Z"
        );
        assert_eq!(format_duration(11215.0), "3 h 06 min");
        assert_eq!(format_duration(59.4), "59 s");
    }
}
