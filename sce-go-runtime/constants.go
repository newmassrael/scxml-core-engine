// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package sce

// W3C SCXML URIs, reserved names, and well-known string literals.
//
// Ports Rust scxml_constants from sce-rust-runtime/src/helpers/scxml_constants.rs.

const (
	// SCXMLEventProcessorType is the SCXML Event I/O Processor URI (W3C SCXML 5.10.2).
	// Default origintype for events sent through the internal engine or between
	// AOT parent/child state machines.
	SCXMLEventProcessorType = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor"

	// BasicHTTPEventProcessorType is the BasicHTTP Event I/O Processor URI (W3C SCXML C.2).
	// Used for <send type="..."> targeting an HTTP URL.
	BasicHTTPEventProcessorType = "http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"

	// InternalTarget is the keyword for events sent to #_internal, placed on the
	// internal queue with high priority (W3C SCXML C.1).
	InternalTarget = "#_internal"

	// ParentTarget is the keyword for child state machines to send events back to
	// their invoking parent (W3C SCXML 6.2).
	ParentTarget = "#_parent"

	// SCXMLSessionTargetPrefix is the session ID target prefix. #_scxml_{sessionid}
	// routes events to a specific session's external queue (W3C SCXML 6.2).
	SCXMLSessionTargetPrefix = "#_scxml_"

	// InvokeTargetPrefix is the invoke target prefix. #_{invokeid} routes events
	// from parent to a specific invoked child state machine (W3C SCXML 6.4).
	InvokeTargetPrefix = "#_"
)
