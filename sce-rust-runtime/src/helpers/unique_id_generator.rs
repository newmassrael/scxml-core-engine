// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Centralized unique ID generation utility.
//!
//! 1:1 port of `sce/include/common/UniqueIdGenerator.h`. Provides thread-safe
//! ID generation for sessions, sends, invokes, events, and other identifiers.
//!
//! Uses `std::sync::atomic` for the global counter, matching the C++ approach
//! of `std::atomic<uint64_t> globalCounter_`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Global counter for ensuring uniqueness within the same millisecond.
static GLOBAL_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate a unique ID with the given prefix.
///
/// Format: `{prefix}_{timestamp}_{counter}`
///
/// Ports C++ `UniqueIdGenerator::generateUniqueId`.
pub fn generate_unique_id(prefix: &str) -> String {
    let timestamp = current_timestamp_millis();
    let counter = GLOBAL_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}_{}{}", prefix, timestamp, counter)
}

/// Generate a unique session ID.
///
/// Ports C++ `UniqueIdGenerator::generateSessionId`.
pub fn generate_session_id(prefix: &str) -> String {
    let prefix = if prefix.is_empty() { "session" } else { prefix };
    generate_unique_id(prefix)
}

/// Generate a unique send ID for event scheduling.
///
/// Ports C++ `UniqueIdGenerator::generateSendId`.
pub fn generate_send_id() -> String {
    generate_unique_id("send")
}

/// Generate a unique invoke ID for SCXML invoke operations.
///
/// W3C SCXML 6.4 (test 224): When `state_id` is provided, the format is
/// `{state_id}.platform_{counter}` for compliance.
///
/// Ports C++ `UniqueIdGenerator::generateInvokeId`.
pub fn generate_invoke_id(state_id: &str) -> String {
    if state_id.is_empty() {
        generate_unique_id("invoke")
    } else {
        let counter = GLOBAL_COUNTER.fetch_add(1, Ordering::Relaxed);
        format!("{}.platform_{}", state_id, counter)
    }
}

/// Generate a unique event ID for HTTP event processing.
///
/// Ports C++ `UniqueIdGenerator::generateEventId`.
pub fn generate_event_id() -> String {
    generate_unique_id("event")
}

/// Generate a unique correlation ID for concurrent operations.
///
/// Ports C++ `UniqueIdGenerator::generateCorrelationId`.
pub fn generate_correlation_id() -> String {
    generate_unique_id("corr")
}

/// Generate a unique action ID for SCXML actions.
///
/// Ports C++ `UniqueIdGenerator::generateActionId`.
pub fn generate_action_id(prefix: &str) -> String {
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
fn current_timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
