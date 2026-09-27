// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! §scxml-6.2: Send action helpers (partial -- static-target subset).
//!
//! Partial port of `sce/include/common/SendHelper.h`. Only the parts needed for
//! static-target send actions are ported here (target validation, routing
//! classification, send ID generation) — the pure subset every profile needs.
//! The C++ SendHelper's remaining responsibilities live in their own
//! `!no_std`-gated modules: parent/child invoke routing in
//! `helpers::invoke_processing`, HTTP form encoding in `helpers::url_encoding`
//! and the `http` module.
//!
//! All validation functions are pure (no side effects, no state).

use crate::helpers::scxml_constants;
use crate::helpers::unique_id_generator;
use crate::SceString;

/// §scxml-6.2 / C.2: A send-target validation failure.
///
/// Returned by [`validate_target`] and [`validate_basic_http_send`]. Each variant
/// is a fieldless discriminant, so the whole error is one byte -- small enough to
/// return by value from a no_std stack frame without tripping
/// `clippy::result_large_err`. (The prior `SceString` payload was a 256-byte
/// `heapless::String` under no_std, which inflated every `Result` returned from
/// these validators.) The human-readable reason a caller raises on
/// `error.execution` / `error.communication` is available via [`Display`](core::fmt::Display);
/// callers that want to embed the offending target value already hold it, so it
/// is not duplicated into the error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendValidationError {
    /// Target value is syntactically invalid (begins with `!`). The caller
    /// raises `error.execution` (§scxml-6.2).
    InvalidTarget,

    /// A BasicHTTP send (`type="BasicHTTPEventProcessor"`) supplied neither
    /// `target` nor `targetexpr`, both of which it requires (§scxml-C-2).
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

/// §scxml-6.2: Check if target is invalid (starts with '!').
///
/// Ports C++ `SendHelper::isInvalidTarget`.
pub fn is_invalid_target(target: &str) -> bool {
    !target.is_empty() && target.starts_with('!')
}

/// §scxml-C-1: Check if target uses the internal event queue.
///
/// Target `#_internal` routes events to the internal queue (high priority).
///
/// Ports C++ `SendHelper::isInternalTarget`.
pub fn is_internal_target(target: &str) -> bool {
    target == scxml_constants::INTERNAL_TARGET
}

/// §scxml-6.4: Check if target is a child invoke session.
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

/// §scxml-6.4: Extract invoke ID from a child target.
///
/// Given `#_{invokeid}`, returns `{invokeid}`.
///
/// Ports C++ `SendHelper::extractInvokeId`.
pub fn extract_invoke_id(target: &str) -> &str {
    target
        .strip_prefix(scxml_constants::INVOKE_TARGET_PREFIX)
        .unwrap_or(target)
}

/// §scxml-C-2: Check if target is an HTTP URL.
///
/// Ports C++ `SendHelper::isHttpTarget`.
pub fn is_http_target(target: &str) -> bool {
    target.starts_with("http://") || target.starts_with("https://")
}

/// §scxml-6.2: Validate send target.
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

/// §scxml-C-1: Check if target is unreachable.
///
/// Empty or `"undefined"` targets indicate unreachable sessions, requiring
/// `error.communication`.
///
/// Ports C++ `SendHelper::isUnreachableTarget`.
pub fn is_unreachable_target(target: &str) -> bool {
    target.is_empty() || target == "undefined"
}

/// §scxml-C-2: Check if send type requires a target attribute.
///
/// BasicHTTP Event I/O Processor requires a target URL.
///
/// Ports C++ `SendHelper::requiresTargetAttribute`.
pub fn requires_target_attribute(send_type: &str) -> bool {
    send_type == scxml_constants::BASIC_HTTP_EVENT_PROCESSOR_TYPE
}

/// §scxml-6.2: Check if send type is supported.
///
/// Supported types: SCXML Event Processor (default), BasicHTTP Event Processor.
///
/// Ports C++ `SendHelper::isSupportedSendType`.
pub fn is_supported_send_type(send_type: &str) -> bool {
    send_type.is_empty()
        || send_type == scxml_constants::SCXML_EVENT_PROCESSOR_TYPE
        || send_type == scxml_constants::BASIC_HTTP_EVENT_PROCESSOR_TYPE
}

/// §scxml-C-2: Validate BasicHTTP send parameters.
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

/// §scxml-6.2: Generate a unique send ID.
///
/// Delegates to [`unique_id_generator::generate_send_id`]. Returns
/// [`SceString`] (= `String` under std, capped `heapless::String` under no_std).
///
/// Ports C++ `SendHelper::generateSendId`.
pub fn generate_send_id() -> SceString {
    unique_id_generator::generate_send_id()
}

/// The largest delay any engine can hold, in milliseconds: the positive range
/// of a signed 64-bit count, because Kotlin's `Long` is signed and every
/// engine answers the same text the same way.
pub const MAX_DELAY_MS: u64 = i64::MAX as u64;

/// §scxml-6.2: a `<send>` delay, read as the CSS2 time the clause names.
///
/// The grammar is ARCHITECTURE.md's "Durations (Single Source of Truth)":
/// surrounding ASCII whitespace aside, a non-negative number (digits with an
/// optional fraction of at least one digit, or a leading `.` and digits; no
/// sign, no exponent) followed directly by `ms` or `s`, either case. The
/// milliseconds are computed in exact decimal and truncated, never through a
/// float. `tests/durations/css2_time.json` holds the cases every engine is
/// measured against.
///
/// Returns `None` when the text is not a time — a bare number included — so
/// the caller raises the argument error rather than choosing a wait.
pub fn parse_delay_to_ms(s: &str) -> Option<u64> {
    let s = s.trim_matches(|c: char| c.is_ascii_whitespace()).as_bytes();
    let (number, scale) = match strip_suffix_ignore_case(s, b"ms") {
        Some(n) => (n, 1u64),
        None => (strip_suffix_ignore_case(s, b"s")?, 1000u64),
    };
    let (whole, fraction) = match number.iter().position(|&b| b == b'.') {
        Some(dot) => (&number[..dot], Some(&number[dot + 1..])),
        None => (number, None),
    };
    // A number is digits, digits "." digits, or "." digits: at least one digit
    // on the side that exists, and the fraction never empty.
    let all_digits = |part: &[u8]| part.iter().all(u8::is_ascii_digit);
    if !all_digits(whole) || fraction.is_some_and(|f| f.is_empty() || !all_digits(f)) {
        return None;
    }
    if whole.is_empty() && fraction.is_none() {
        return None;
    }
    let mut ms: u64 = 0;
    for &digit in whole {
        ms = ms.checked_mul(10)?.checked_add(u64::from(digit - b'0'))?;
    }
    ms = ms.checked_mul(scale)?;
    // Only the fraction digits that name whole milliseconds count; the rest
    // truncate.
    let mut place = scale / 10;
    for &digit in fraction.unwrap_or(&[]) {
        if place == 0 {
            break;
        }
        ms = ms.checked_add(u64::from(digit - b'0') * place)?;
        place /= 10;
    }
    (ms <= MAX_DELAY_MS).then_some(ms)
}

fn strip_suffix_ignore_case<'a>(text: &'a [u8], suffix: &[u8]) -> Option<&'a [u8]> {
    let split = text.len().checked_sub(suffix.len())?;
    text[split..]
        .eq_ignore_ascii_case(suffix)
        .then(|| &text[..split])
}

#[cfg(all(test, not(feature = "no_std")))]
mod duration_table {
    use super::parse_delay_to_ms;
    use crate::json::{parse, Value};

    fn member<'a>(fields: &'a [(String, Value)], key: &str) -> &'a Value {
        &fields
            .iter()
            .find(|(k, _)| k == key)
            .unwrap_or_else(|| panic!("a case has `{key}`"))
            .1
    }

    /// tests/durations/css2_time.json: the cases every engine's delay reader
    /// is measured against, read with this crate's own JSON parser.
    #[test]
    fn a_delay_is_read_as_the_one_css2_time_every_engine_reads() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tests/durations/css2_time.json"
        );
        let table = std::fs::read_to_string(path).expect("the shared duration table");
        let Value::Object(members) = parse(&table).expect("the table is JSON") else {
            panic!("the table is an object");
        };
        let Some((_, Value::Array(cases))) = members.iter().find(|(key, _)| key == "cases") else {
            panic!("the table has cases");
        };
        assert!(cases.len() >= 20, "the table lost cases: {}", cases.len());
        for case in cases {
            let Value::Object(fields) = case else {
                panic!("a case is an object");
            };
            let Value::Text(name) = member(fields, "name") else {
                panic!("a case's name is text");
            };
            let Value::Text(text) = member(fields, "text") else {
                panic!("{name}: text is text");
            };
            let want = match member(fields, "ms") {
                Value::Null => None,
                Value::Number(n) => Some(n.parse::<u64>().expect("ms is a count")),
                other => panic!("{name}: ms is a count or null, got {other:?}"),
            };
            assert_eq!(parse_delay_to_ms(text), want, "{name}: {text:?}");
        }
    }
}
