// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The time a save is stamped with, behind a trait so a test chooses it.
//!
//! A revision's identity never depends on the time (it is the digest of the
//! text), so a clock only labels history for a person to read. That is why it is
//! injected and not read from the system inside the store: a test that wants a
//! known history gives it one instead of sleeping.

use std::time::{SystemTime, UNIX_EPOCH};

/// A source of "now" as an RFC 3339 UTC timestamp.
pub trait Clock {
    fn now(&self) -> String;
}

/// The system clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> String {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        rfc3339_utc(seconds)
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

/// `seconds` since the Unix epoch as `YYYY-MM-DDTHH:MM:SSZ`.
///
/// The civil-date step is Howard Hinnant's days-to-date algorithm, which is
/// exact for the proleptic Gregorian calendar and needs no date library for the
/// one conversion this crate makes.
pub fn rfc3339_utc(seconds: u64) -> String {
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
