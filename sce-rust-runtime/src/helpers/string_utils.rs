// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! String utilities for SCXML event classification.
//!
//! 1:1 port of `sce/include/common/StringUtils.h`. Provides fast prefix
//! detection for platform event names (`done.*`, `error.*`).

/// W3C SCXML 5.10.1: Check if an event name is a platform event.
///
/// Platform events are those with `done.` or `error.` prefixes.
///
/// Ports C++ `SCE::isPlatformEvent`.
///
/// # Examples
///
/// ```
/// use sce_rust_runtime::helpers::string_utils::is_platform_event;
///
/// assert!(is_platform_event("done.state.s1"));
/// assert!(is_platform_event("error.execution"));
/// assert!(!is_platform_event("my.event"));
/// ```
#[inline]
pub fn is_platform_event(event_name: &str) -> bool {
    event_name.starts_with("done.") || event_name.starts_with("error.")
}

/// Check if a string starts with a given prefix.
///
/// Convenience wrapper for use in generated code (matches C++ `detail::starts_with`).
#[inline]
pub fn starts_with(s: &str, prefix: &str) -> bool {
    s.starts_with(prefix)
}
