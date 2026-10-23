// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML URIs, reserved names and well-known string literals.
//
// Ports Rust `scxml_constants` from
// `backends/rust/runtime/src/helpers/scxml_constants.rs`.

/// The SCXML Event I/O Processor URI (§scxml-C-1). The default origintype for
/// events sent through the internal engine or between AOT parent and child state
/// machines.
public let scxmlEventProcessorType = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor"

/// The BasicHTTP Event I/O Processor URI (§scxml-C-2), used for
/// `<send type="...">` targeting an HTTP URL.
public let basicHTTPEventProcessorType = "http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"

/// The keyword for events sent to `#_internal`, placed on the internal queue with
/// high priority (§scxml-C-1).
public let internalTarget = "#_internal"

/// The keyword for child state machines to send events back to their invoking
/// parent (§scxml-6.2).
public let parentTarget = "#_parent"

/// The session id target prefix. `#_scxml_{sessionid}` routes events to a
/// specific session's external queue (§scxml-6.2).
public let scxmlSessionTargetPrefix = "#_scxml_"

/// The invoke target prefix. `#_{invokeid}` routes events from parent to a
/// specific invoked child state machine (§scxml-6.4).
public let invokeTargetPrefix = "#_"
