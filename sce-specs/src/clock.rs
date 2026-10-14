// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The time a save is stamped with, behind a trait so a test chooses it.
//!
//! A revision's identity never depends on the time (it is the digest of the
//! text), so a clock only labels history for a person to read. That is why it is
//! injected and not read from the system inside the store: a test that wants a
//! known history gives it one instead of sleeping.
//!
//! A request's lease is the one place the time decides something and not only
//! labels it: a lease runs out. The clock says that as seconds too
//! ([`Clock::epoch`]), and a test that wants a lease to run out moves a
//! [`ManualClock`] instead of waiting for it.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// A source of "now" as an RFC 3339 UTC timestamp.
pub trait Clock {
    fn now(&self) -> String;

    /// The same moment as seconds since the Unix epoch, for arithmetic on it. A clock
    /// that only knows how to say it as text is read back from the text; one that has
    /// the seconds says them.
    fn epoch(&self) -> u64 {
        parse_utc_timestamp(&self.now()).unwrap_or(0)
    }
}

/// The system clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> String {
        utc_timestamp(self.epoch())
    }

    fn epoch(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

/// A clock a test moves by hand: it says the same moment until it is told to say another.
/// Shared between the store and the test by reference, so the test can advance it while the
/// store holds it.
#[derive(Debug, Default)]
pub struct ManualClock(AtomicU64);

impl ManualClock {
    /// A clock saying `epoch` seconds after the Unix epoch.
    pub fn at(epoch: u64) -> Self {
        ManualClock(AtomicU64::new(epoch))
    }

    /// Move the clock `seconds` forward.
    pub fn advance(&self, seconds: u64) {
        self.0.fetch_add(seconds, Ordering::SeqCst);
    }

    /// Say `epoch` from now on.
    pub fn set(&self, epoch: u64) {
        self.0.store(epoch, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> String {
        utc_timestamp(self.epoch())
    }

    fn epoch(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

impl<C: Clock + ?Sized> Clock for &C {
    fn now(&self) -> String {
        (**self).now()
    }

    fn epoch(&self) -> u64 {
        (**self).epoch()
    }
}

/// A clock shared with a test (or between threads) through an `Arc`.
impl<C: Clock + ?Sized> Clock for std::sync::Arc<C> {
    fn now(&self) -> String {
        (**self).now()
    }

    fn epoch(&self) -> u64 {
        (**self).epoch()
    }
}

/// A clock that always says the same thing, for tests.
#[derive(Debug, Clone)]
pub struct FixedClock(pub String);

impl Clock for FixedClock {
    fn now(&self) -> String {
        self.0.clone()
    }
}

/// `seconds` since the Unix epoch as `YYYY-MM-DDTHH:MM:SSZ` (the profile of
/// RFC 3339 that every JSON reader and `Date` parse accepts).
///
/// The civil-date step is Howard Hinnant's days-to-date algorithm, which is
/// exact for the proleptic Gregorian calendar and needs no date library for the
/// one conversion this crate makes.
pub fn utc_timestamp(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let in_day = seconds % 86_400;
    let (hour, minute, second) = (in_day / 3_600, (in_day % 3_600) / 60, in_day % 60);

    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = if month <= 2 { year + 1 } else { year };

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// The seconds since the Unix epoch of a `YYYY-MM-DDTHH:MM:SSZ` timestamp, which is what
/// [`utc_timestamp`] writes; `None` for anything else, a date that does not exist
/// included (`2026-02-30`) and one before 1970.
///
/// The date step is the other half of Hinnant's pair (civil date to days). The result is
/// written back and compared with the text, so a form this reads and `utc_timestamp` would
/// not write is refused rather than read as some other moment.
pub fn parse_utc_timestamp(text: &str) -> Option<u64> {
    let bytes = text.as_bytes();
    let shape_ok = bytes.len() == 20
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[13] == b':'
        && bytes[16] == b':'
        && bytes[19] == b'Z';
    if !shape_ok {
        return None;
    }
    let field = |from: usize, to: usize| -> Option<i64> {
        let digits = text.get(from..to)?;
        digits
            .bytes()
            .all(|b| b.is_ascii_digit())
            .then(|| digits.parse().ok())
            .flatten()
    };
    let (year, month, day) = (field(0, 4)?, field(5, 7)?, field(8, 10)?);
    let (hour, minute, second) = (field(11, 13)?, field(14, 16)?, field(17, 19)?);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let month_index = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    let seconds = days
        .checked_mul(86_400)?
        .checked_add(hour * 3_600 + minute * 60 + second)?;
    let seconds = u64::try_from(seconds).ok()?;
    (utc_timestamp(seconds) == text).then_some(seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_timestamp_written_is_read_back_as_the_seconds_it_was_written_from() {
        for seconds in [
            0,
            59,
            86_399,
            86_400,
            951_782_400,   // 2000-02-29, a leap day
            1_709_164_800, // 2024-02-29
            4_102_444_800, // 2100-01-01, not a leap year
            1_791_000_123,
        ] {
            assert_eq!(parse_utc_timestamp(&utc_timestamp(seconds)), Some(seconds));
        }
    }

    #[test]
    fn what_utc_timestamp_would_not_write_is_not_read() {
        for text in [
            "",
            "garbage",
            "2026-02-30T00:00:00Z",   // no such day
            "2026-13-01T00:00:00Z",   // no such month
            "2026-10-05T24:00:00Z",   // no such hour
            "2026-10-05T00:60:00Z",   // no such minute
            "2026-10-05 00:00:00Z",   // not the T form
            "2026-10-05T00:00:00",    // no Z
            "2026-10-05T00:00:00+09", // an offset, not UTC
            "1969-12-31T23:59:59Z",   // before the epoch
            "2026-1O-05T00:00:00Z",   // a letter among the digits
            "2026-10-05T00:00:-1Z",   // a sign
        ] {
            assert_eq!(parse_utc_timestamp(text), None, "{text}");
        }
    }

    #[test]
    fn a_clock_that_only_says_the_text_is_read_back_for_its_seconds() {
        let fixed = FixedClock("2026-10-05T09:00:00Z".to_string());
        assert_eq!(fixed.epoch(), 1_791_190_800);
        assert_eq!(FixedClock("not a time".to_string()).epoch(), 0);
    }

    #[test]
    fn a_manual_clock_says_the_moment_it_was_given_and_moves_when_told_to() {
        let clock = ManualClock::at(1_000);
        assert_eq!(clock.epoch(), 1_000);
        clock.advance(30);
        assert_eq!(clock.epoch(), 1_030);
        assert_eq!(clock.now(), utc_timestamp(1_030));
        clock.set(5);
        assert_eq!(clock.epoch(), 5);
    }

    #[test]
    fn a_clock_held_by_reference_is_the_clock_it_refers_to() {
        let clock = ManualClock::at(100);
        let held: &ManualClock = &clock;
        clock.advance(7);
        assert_eq!(Clock::epoch(&held), 107);
    }
}
