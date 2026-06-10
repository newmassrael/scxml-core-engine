// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML URIs, reserved names, and well-known string literals.
//!
//! Ports C++ `sce/include/common/SCXMLConstants.h`. Values are compile-time
//! constants so the generated code can reference them without allocations.

// ── W3C SCXML I/O Processor URIs (`_event.origintype` for outbound sends) ──

/// SCXML Event I/O Processor (W3C SCXML 5.10.2). Default `origintype` for
/// events sent through the internal engine or between AOT parent/child SMs.
pub const SCXML_EVENT_PROCESSOR_TYPE: &str = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor";

/// BasicHTTP Event I/O Processor (W3C SCXML C.2). Used for `<send type="...">`
/// targeting an HTTP URL via `HttpSendRequest` (`!no_std`-gated `http` module).
pub const BASIC_HTTP_EVENT_PROCESSOR_TYPE: &str =
    "http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor";

/// Internal target keyword — events sent to `#_internal` are placed on the
/// internal queue with high priority (W3C SCXML C.1).
pub const INTERNAL_TARGET: &str = "#_internal";

/// Parent target keyword — child state machines use `#_parent` to send events
/// back to their invoking parent (W3C SCXML 6.2).
pub const PARENT_TARGET: &str = "#_parent";

/// Session ID target prefix — `#_scxml_{sessionid}` routes events to a
/// specific session's external queue (W3C SCXML 6.2).
pub const SCXML_SESSION_TARGET_PREFIX: &str = "#_scxml_";

/// Invoke target prefix — `#_{invokeid}` routes events from parent to a specific
/// invoked child state machine (W3C SCXML 6.4).
pub const INVOKE_TARGET_PREFIX: &str = "#_";
