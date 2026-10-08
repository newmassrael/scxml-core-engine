// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Centralized unique ID generation utility.
//!
//! 1:1 port of `sce/include/common/UniqueIdGenerator.h`. Provides thread-safe
//! ID generation for sessions, sends, invokes, events, and other identifiers.
//!
//! Uses `std::sync::atomic` for the global counter, matching the C++ approach
//! of `std::atomic<uint64_t> globalCounter_`.
//!
//! SCE Protocol-Synthesis RFC §synth-5-J-2 (lines 1989-1994): under the no_std variant the
//! Unix-epoch timestamp source (`SystemTime::now().duration_since(UNIX_EPOCH)`)
//! is unavailable. Send/invoke/event IDs only need uniqueness within a single
//! statechart instance (§scxml-5.10.1) — the `AtomicU64` counter alone
//! satisfies that contract, so under `--features=no_std` the timestamp branch
//! returns zero and uniqueness rides entirely on the global counter. ID
//! strings are returned as [`SceString`] (= `String` under std, capped
//! `heapless::String<MAX_EVENT_STRING_LEN>` under no_std).
//!
//! ## 64-bit atomics on platforms without native support
//!
//! Cortex-M4 (the canonical MCU target `thumbv7em-none-eabihf`) lacks native
//! 64-bit atomic instructions and `core::sync::atomic::AtomicU64` is therefore
//! absent from its `core`. To keep the u64 ID semantics identical under std
//! and no_std, the counter rides on [`portable_atomic::AtomicU64`] — on host
//! targets this is a re-export of the native `core` atomic (zero overhead),
//! and on MCU targets it is the crate's `fallback` lock-based emulation. This
//! is the standard embedded-Rust convention for wider-than-native atomics
//! (matches `embassy`, `defmt`, et al.).

use crate::SceString;
use core::sync::atomic::Ordering;
use portable_atomic::AtomicU64;
#[cfg(all(
    not(feature = "no_std"),
    not(all(target_arch = "wasm32", target_os = "unknown"))
))]
use std::time::{SystemTime, UNIX_EPOCH};

/// Global counter for ensuring uniqueness within the same millisecond.
///
/// Backed by [`portable_atomic::AtomicU64`] so the same `u64` ID semantics
/// hold under std and on MCU targets that lack native 64-bit atomics (see
/// module-level note).
static GLOBAL_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Build `{prefix}_{timestamp}{counter}` into a fresh [`SceString`].
///
/// Single formatter shared by all generators below — std uses `format!`,
/// no_std uses `core::fmt::Write` into a `heapless::String`. Overflow under
/// no_std truncates per heapless semantics (the 256-byte cap is far above
/// any realistic prefix + u64 + u64).
fn format_unique_id(prefix: &str, timestamp: u64, counter: u64) -> SceString {
    #[cfg(not(feature = "no_std"))]
    {
        format!("{}_{}{}", prefix, timestamp, counter)
    }
    #[cfg(feature = "no_std")]
    {
        use core::fmt::Write;
        let mut s = SceString::new();
        let _ = write!(&mut s, "{}_{}{}", prefix, timestamp, counter);
        s
    }
}

/// Build `{state_id}.platform_{counter}` into a fresh [`SceString`].
///
/// §scxml-6.4 (test 224) invoke ID layout when a state_id is supplied.
fn format_invoke_id(state_id: &str, counter: u64) -> SceString {
    #[cfg(not(feature = "no_std"))]
    {
        format!("{}.platform_{}", state_id, counter)
    }
    #[cfg(feature = "no_std")]
    {
        use core::fmt::Write;
        let mut s = SceString::new();
        let _ = write!(&mut s, "{}.platform_{}", state_id, counter);
        s
    }
}

/// Build `_auto_send_{number}` into a fresh [`SceString`]: the id a machine
/// gives a `<send idlocation>`, from the number of ids it has generated so far.
///
/// The same text on every backend (`_auto_send_` and a decimal number counted
/// from one), so a machine of a document hands the document the same id
/// wherever it runs and a saved state carries on from the number alone. At most
/// 31 bytes: [`AUTO_SEND_ID_MAX_LEN`].
pub fn format_auto_send_id(number: u64) -> SceString {
    #[cfg(not(feature = "no_std"))]
    {
        format!("_auto_send_{}", number)
    }
    #[cfg(feature = "no_std")]
    {
        use core::fmt::Write;
        let mut s = SceString::new();
        let _ = write!(&mut s, "_auto_send_{}", number);
        s
    }
}

/// The most bytes an id from [`format_auto_send_id`] takes: `_auto_send_` and
/// the twenty digits of the largest `u64`. A string a document stores the id in
/// must hold this many.
pub const AUTO_SEND_ID_MAX_LEN: usize = 31;

/// Generate a unique ID with the given prefix.
///
/// Format: `{prefix}_{timestamp}_{counter}`
///
/// Ports C++ `UniqueIdGenerator::generateUniqueId`.
pub fn generate_unique_id(prefix: &str) -> SceString {
    let timestamp = current_timestamp_millis();
    let counter = GLOBAL_COUNTER.fetch_add(1, Ordering::Relaxed);
    format_unique_id(prefix, timestamp, counter)
}

/// Generate a unique session ID.
///
/// Ports C++ `UniqueIdGenerator::generateSessionId`.
pub fn generate_session_id(prefix: &str) -> SceString {
    let prefix = if prefix.is_empty() { "session" } else { prefix };
    generate_unique_id(prefix)
}

/// Generate a unique send ID for event scheduling.
///
/// Ports C++ `UniqueIdGenerator::generateSendId`.
pub fn generate_send_id() -> SceString {
    generate_unique_id("send")
}

/// Generate a unique invoke ID for SCXML invoke operations.
///
/// §scxml-6.4 (test 224): When `state_id` is provided, the format is
/// `{state_id}.platform_{counter}` for compliance.
///
/// Ports C++ `UniqueIdGenerator::generateInvokeId`.
pub fn generate_invoke_id(state_id: &str) -> SceString {
    if state_id.is_empty() {
        generate_unique_id("invoke")
    } else {
        let counter = GLOBAL_COUNTER.fetch_add(1, Ordering::Relaxed);
        format_invoke_id(state_id, counter)
    }
}

/// Generate a unique event ID for HTTP event processing.
///
/// Ports C++ `UniqueIdGenerator::generateEventId`.
pub fn generate_event_id() -> SceString {
    generate_unique_id("event")
}

/// Generate a unique correlation ID for concurrent operations.
///
/// Ports C++ `UniqueIdGenerator::generateCorrelationId`.
pub fn generate_correlation_id() -> SceString {
    generate_unique_id("corr")
}

/// Generate a unique action ID for SCXML actions.
///
/// Ports C++ `UniqueIdGenerator::generateActionId`.
pub fn generate_action_id(prefix: &str) -> SceString {
    let prefix = if prefix.is_empty() { "action" } else { prefix };
    generate_unique_id(prefix)
}

/// Generate a numeric session ID (for legacy compatibility).
///
/// Ports C++ `UniqueIdGenerator::generateNumericSessionId`.
pub fn generate_numeric_session_id() -> u64 {
    let timestamp = current_timestamp_millis();
    let counter = GLOBAL_COUNTER.fetch_add(1, Ordering::Relaxed);
    timestamp.wrapping_mul(1000).wrapping_add(counter)
}

/// Reset internal counters (for testing purposes only).
///
/// Ports C++ `UniqueIdGenerator::resetForTesting`.
pub fn reset_for_testing() {
    GLOBAL_COUNTER.store(0, Ordering::Relaxed);
}

/// Get current timestamp in milliseconds since Unix epoch.
///
/// SCE Protocol-Synthesis RFC §synth-5-J-2: under `--features=no_std` the Unix-epoch source is
/// unavailable; the timestamp segment is reduced to zero and per-instance
/// uniqueness is provided by `GLOBAL_COUNTER`. The same holds on
/// `wasm32-unknown-unknown`, which provides no wall clock either: an id's
/// uniqueness never rested on its timestamp, so the id is built without one
/// rather than from a clock that would stop the module.
// Reads the OS wall clock, and is absent where there is none (clippy.toml).
#[allow(clippy::disallowed_methods)]
#[cfg(all(
    not(feature = "no_std"),
    not(all(target_arch = "wasm32", target_os = "unknown"))
))]
fn current_timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(any(feature = "no_std", all(target_arch = "wasm32", target_os = "unknown")))]
fn current_timestamp_millis() -> u64 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_send_id_is_the_prefix_and_the_number_it_was_counted_as() {
        assert_eq!(format_auto_send_id(1).as_str(), "_auto_send_1");
        assert_eq!(format_auto_send_id(10).as_str(), "_auto_send_10");
        assert_eq!(
            format_auto_send_id(i64::MAX as u64).as_str(),
            "_auto_send_9223372036854775807"
        );
    }

    #[test]
    fn no_count_makes_an_id_longer_than_the_length_a_document_is_told_to_hold() {
        assert_eq!(format_auto_send_id(u64::MAX).len(), AUTO_SEND_ID_MAX_LEN);
        assert_eq!(format_auto_send_id(0).len(), "_auto_send_0".len());
    }
}
