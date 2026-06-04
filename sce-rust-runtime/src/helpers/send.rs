// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML 6.2: Send action helpers (partial -- static-target subset).
//!
//! Partial port of `sce/include/common/SendHelper.h`. Only the parts needed for
//! static-target send actions are ported here (target validation, routing
//! classification, send ID generation). The full 21.8K SendHelper with
//! parent/child invoke routing and HTTP form encoding is deferred to Phase 3/4.
//!
//! All validation functions are pure (no side effects, no state).

use crate::helpers::scxml_constants;
use crate::helpers::unique_id_generator;
use crate::SceString;

/// W3C SCXML 6.2 / C.2: A send-target validation failure.
///
/// Returned by [`validate_target`] and [`validate_basic_http_send`]. Each variant
/// is a fieldless discriminant, so the whole error is one byte -- small enough to
/// return by value from a no_std stack frame without tripping
/// `clippy::result_large_err`. (The prior `SceString` payload was a 256-byte
/// `heapless::String` under no_std, which inflated every `Result` returned from
/// these validators.) The human-readable reason a caller raises on
/// `error.execution` / `error.communication` is available via [`Display`];
/// callers that want to embed the offending target value already hold it, so it
/// is not duplicated into the error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendValidationError {
    /// Target value is syntactically invalid (begins with `!`). The caller
    /// raises `error.execution` (W3C SCXML 6.2).
    InvalidTarget,

    /// A BasicHTTP send (`type="BasicHTTPEventProcessor"`) supplied neither
    /// `target` nor `targetexpr`, both of which it requires (W3C SCXML C.2).
    MissingHttpTarget,
}

impl core::fmt::Display for SendValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::InvalidTarget => "Invalid target value",
            Self::MissingHttpTarget => "BasicHTTPEventProcessor requires target attribute",
        })
    }
}

/// W3C SCXML 6.2: Check if target is invalid (starts with '!').
///
/// Ports C++ `SendHelper::isInvalidTarget`.
pub fn is_invalid_target(target: &str) -> bool {
    !target.is_empty() && target.starts_with('!')
}

/// W3C SCXML C.1: Check if target uses the internal event queue.
///
/// Target `#_internal` routes events to the internal queue (high priority).
///
/// Ports C++ `SendHelper::isInternalTarget`.
pub fn is_internal_target(target: &str) -> bool {
    target == scxml_constants::INTERNAL_TARGET
}

/// W3C SCXML 6.4: Check if target is a child invoke session.
///
/// Targets matching `#_{invokeid}` (but not `#_parent`, `#_internal`, or
/// `#_scxml_*`) are child invoke targets.
///
/// Ports C++ `SendHelper::isChildInvokeTarget`.
pub fn is_child_invoke_target(target: &str) -> bool {
    if !target.starts_with(scxml_constants::INVOKE_TARGET_PREFIX) {
        return false;
    }
    // Exclude special reserved targets
    if target == scxml_constants::PARENT_TARGET || target == scxml_constants::INTERNAL_TARGET {
        return false;
    }
    // Exclude SCXML session targets
    if target.starts_with(scxml_constants::SCXML_SESSION_TARGET_PREFIX) {
        return false;
    }
    true
}

/// W3C SCXML 6.4: Extract invoke ID from a child target.
///
/// Given `#_{invokeid}`, returns `{invokeid}`.
///
/// Ports C++ `SendHelper::extractInvokeId`.
pub fn extract_invoke_id(target: &str) -> &str {
    target
        .strip_prefix(scxml_constants::INVOKE_TARGET_PREFIX)
        .unwrap_or(target)
}

/// W3C SCXML C.2: Check if target is an HTTP URL.
///
/// Ports C++ `SendHelper::isHttpTarget`.
pub fn is_http_target(target: &str) -> bool {
    target.starts_with("http://") || target.starts_with("https://")
}

/// W3C SCXML 6.2: Validate send target.
///
/// Returns `Ok(())` if valid, `Err(SendValidationError::InvalidTarget)` if invalid
/// (the caller raises `error.execution`).
///
/// Ports C++ `SendHelper::validateTarget`.
pub fn validate_target(target: &str) -> Result<(), SendValidationError> {
    if is_invalid_target(target) {
        Err(SendValidationError::InvalidTarget)
    } else {
        Ok(())
    }
}

/// W3C SCXML C.1: Check if target is unreachable.
///
/// Empty or `"undefined"` targets indicate unreachable sessions, requiring
/// `error.communication`.
///
/// Ports C++ `SendHelper::isUnreachableTarget`.
pub fn is_unreachable_target(target: &str) -> bool {
    target.is_empty() || target == "undefined"
}

/// W3C SCXML C.2: Check if send type requires a target attribute.
///
/// BasicHTTP Event I/O Processor requires a target URL.
///
/// Ports C++ `SendHelper::requiresTargetAttribute`.
pub fn requires_target_attribute(send_type: &str) -> bool {
    send_type == scxml_constants::BASIC_HTTP_EVENT_PROCESSOR_TYPE
}

/// W3C SCXML 6.2: Check if send type is supported.
///
/// Supported types: SCXML Event Processor (default), BasicHTTP Event Processor.
///
/// Ports C++ `SendHelper::isSupportedSendType`.
pub fn is_supported_send_type(send_type: &str) -> bool {
    send_type.is_empty()
        || send_type == scxml_constants::SCXML_EVENT_PROCESSOR_TYPE
        || send_type == scxml_constants::BASIC_HTTP_EVENT_PROCESSOR_TYPE
}

/// W3C SCXML C.2: Validate BasicHTTP send parameters.
///
/// Returns `Ok(())` if valid, `Err(SendValidationError::MissingHttpTarget)` if a
/// target is required but neither `target` nor `targetexpr` was supplied.
///
/// Ports C++ `SendHelper::validateBasicHttpSend`.
pub fn validate_basic_http_send(
    send_type: &str,
    target: &str,
    target_expr: &str,
) -> Result<(), SendValidationError> {
    if requires_target_attribute(send_type) && target.is_empty() && target_expr.is_empty() {
        Err(SendValidationError::MissingHttpTarget)
    } else {
        Ok(())
    }
}

/// W3C SCXML 6.2: Generate a unique send ID.
///
/// Delegates to [`unique_id_generator::generate_send_id`]. Returns
/// [`SceString`] (= `String` under std, capped `heapless::String` under no_std).
///
/// Ports C++ `SendHelper::generateSendId`.
pub fn generate_send_id() -> SceString {
    unique_id_generator::generate_send_id()
}

/// W3C SCXML 6.2: Parse a CSS2-style time duration into milliseconds.
///
/// Accepts the same surface syntax as C++ `parseDelayToMs`:
/// - `"1s"`, `"1.5s"` → seconds (converted to ms)
/// - `"250ms"` → milliseconds
/// - bare number `"500"` → milliseconds
///
/// Returns `None` on unparseable input; callers typically default to `0` and
/// may raise `error.execution` (matches C++ behavior).
pub fn parse_delay_to_ms(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return Some(0);
    }
    // Check suffix (order matters: "ms" before "s").
    if let Some(num) = s.strip_suffix("ms") {
        num.trim().parse::<f64>().ok().map(|n| n.max(0.0) as u64)
    } else if let Some(num) = s.strip_suffix('s') {
        num.trim()
            .parse::<f64>()
            .ok()
            .map(|n| (n.max(0.0) * 1000.0) as u64)
    } else {
        // Bare number: interpret as milliseconds (matches C++ fallback).
        s.parse::<f64>().ok().map(|n| n.max(0.0) as u64)
    }
}
