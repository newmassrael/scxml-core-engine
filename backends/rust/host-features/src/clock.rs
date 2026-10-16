// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The clock (docs/SCE_HOST_FEATURES.md, section 2).

use crate::{failure, record, Outcome, ReferenceHost, Transport};
use serde_json::json;

/// Both readings of one instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reading {
    /// Milliseconds since 1970-01-01T00:00:00Z, POSIX time.
    pub utc_ms: i64,
    /// Milliseconds since an origin fixed for the life of the process.
    pub monotonic_ms: i64,
}

/// The time zone a host reads local time in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zone {
    /// An IANA identifier, or empty when only an offset is known.
    pub id: String,
    /// Seconds east of UTC in force now.
    pub utc_offset_s: i32,
}

/// Where a host's time comes from. A source that cannot be read answers `None`,
/// which the host reports as `unavailable`.
pub trait ClockSource {
    fn read(&mut self) -> Option<Reading>;
    fn zone(&mut self) -> Option<Zone>;
}

/// The operating system's clock.
pub struct SystemClock {
    origin: std::time::Instant,
}

impl SystemClock {
    pub fn new() -> Self {
        Self {
            origin: std::time::Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl ClockSource for SystemClock {
    fn read(&mut self) -> Option<Reading> {
        let since_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?;
        Some(Reading {
            utc_ms: i64::try_from(since_epoch.as_millis()).ok()?,
            monotonic_ms: i64::try_from(self.origin.elapsed().as_millis()).ok()?,
        })
    }

    #[cfg(unix)]
    fn zone(&mut self) -> Option<Zone> {
        // The offset in force now, from the C library's local time; the
        // identifier is the TZ variable when it names a zone, which the library
        // has no way to report.
        let now = unsafe { libc::time(std::ptr::null_mut()) };
        let mut local: libc::tm = unsafe { std::mem::zeroed() };
        if unsafe { libc::localtime_r(&now, &mut local) }.is_null() {
            return None;
        }
        let id = std::env::var("TZ")
            .ok()
            .map(|tz| tz.trim_start_matches(':').to_string())
            .filter(|tz| tz.contains('/') || tz == "UTC")
            .unwrap_or_default();
        Some(Zone {
            id,
            utc_offset_s: i32::try_from(local.tm_gmtoff).ok()?,
        })
    }

    #[cfg(not(unix))]
    fn zone(&mut self) -> Option<Zone> {
        None
    }
}

/// A clock a scenario sets: it reads what it was told.
#[derive(Debug, Clone, Default)]
pub struct ScriptedClock {
    pub reading: Option<Reading>,
    pub zone: Option<Zone>,
}

impl ScriptedClock {
    pub fn new() -> Self {
        Self {
            reading: Some(Reading {
                utc_ms: 0,
                monotonic_ms: 0,
            }),
            zone: Some(Zone {
                id: String::new(),
                utc_offset_s: 0,
            }),
        }
    }
}

impl ClockSource for ScriptedClock {
    fn read(&mut self) -> Option<Reading> {
        self.reading
    }

    fn zone(&mut self) -> Option<Zone> {
        self.zone.clone()
    }
}

/// A clock a scenario shares with the host it sets: both hold the one clock.
impl ClockSource for std::sync::Arc<std::sync::Mutex<ScriptedClock>> {
    fn read(&mut self) -> Option<Reading> {
        self.lock()
            .expect("the clock is not left mid-update")
            .reading
    }

    fn zone(&mut self) -> Option<Zone> {
        self.lock()
            .expect("the clock is not left mid-update")
            .zone
            .clone()
    }
}

pub(crate) fn now<T: Transport>(host: &mut ReferenceHost<T>) -> Outcome {
    let Some(reading) = host.clock.read() else {
        return failure("sce-host-clock-now", "unavailable");
    };
    // No answer goes below one already given.
    host.monotonic_floor = host.monotonic_floor.max(reading.monotonic_ms);
    record(json!({
        "utc_ms": reading.utc_ms.to_string(),
        "monotonic_ms": host.monotonic_floor.to_string(),
    }))
}

pub(crate) fn zone<T: Transport>(host: &mut ReferenceHost<T>) -> Outcome {
    match host.clock.zone() {
        Some(zone) => record(json!({
            "zone_id": zone.id,
            "utc_offset_s": zone.utc_offset_s,
        })),
        None => failure("sce-host-clock-zone", "unavailable"),
    }
}

/// The instant a notification is compared with: the host's UTC reading, or
/// `None` when the clock cannot be read.
pub(crate) fn utc_now<T: Transport>(host: &mut ReferenceHost<T>) -> Option<i64> {
    host.clock.read().map(|reading| reading.utc_ms)
}
