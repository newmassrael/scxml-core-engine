// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package sce

import (
	"fmt"
	"sync/atomic"
	"time"
)

// Centralized unique ID generation utility.
//
// 1:1 port of Rust unique_id_generator from
// backends/rust/runtime/src/helpers/unique_id_generator.rs. Provides thread-safe ID
// generation for sessions, sends, invokes, events, and other identifiers.

// globalCounter ensures uniqueness within the same millisecond.
var globalCounter atomic.Uint64

// currentTimestampMillis returns the current time in milliseconds since Unix epoch.
func currentTimestampMillis() uint64 {
	return uint64(time.Now().UnixMilli())
}

// GenerateUniqueID generates a unique ID with the given prefix.
//
// Format: {prefix}_{timestamp}{counter}
//
// Ports Rust unique_id_generator::generate_unique_id.
func GenerateUniqueID(prefix string) string {
	timestamp := currentTimestampMillis()
	counter := globalCounter.Add(1) - 1
	return fmt.Sprintf("%s_%d%d", prefix, timestamp, counter)
}

// GenerateSessionID generates a unique session ID.
//
// Ports Rust unique_id_generator::generate_session_id.
func GenerateSessionID(prefix ...string) string {
	p := "session"
	if len(prefix) > 0 && prefix[0] != "" {
		p = prefix[0]
	}
	return GenerateUniqueID(p)
}

// GenerateSendID generates a unique send ID for event scheduling.
//
// Ports Rust unique_id_generator::generate_send_id.
func GenerateSendID() string {
	return GenerateUniqueID("send")
}

// GenerateInvokeID generates a unique invoke ID for SCXML invoke operations.
//
// W3C SCXML 6.4 (test 224): When stateID is provided, the format is
// "{stateID}.platform_{counter}" for compliance.
//
// Ports Rust unique_id_generator::generate_invoke_id.
func GenerateInvokeID(stateID ...string) string {
	sid := ""
	if len(stateID) > 0 {
		sid = stateID[0]
	}
	if sid == "" {
		return GenerateUniqueID("invoke")
	}
	counter := globalCounter.Add(1) - 1
	return fmt.Sprintf("%s.platform_%d", sid, counter)
}

// GenerateEventID generates a unique event ID for HTTP event processing.
//
// Ports Rust unique_id_generator::generate_event_id.
func GenerateEventID() string {
	return GenerateUniqueID("event")
}

// GenerateCorrelationID generates a unique correlation ID for concurrent operations.
//
// Ports Rust unique_id_generator::generate_correlation_id.
func GenerateCorrelationID() string {
	return GenerateUniqueID("corr")
}

// GenerateActionID generates a unique action ID for SCXML actions.
//
// Ports Rust unique_id_generator::generate_action_id.
func GenerateActionID(prefix string) string {
	if prefix == "" {
		prefix = "action"
	}
	return GenerateUniqueID(prefix)
}

// GenerateNumericSessionID generates a numeric session ID (for legacy compatibility).
//
// Ports Rust unique_id_generator::generate_numeric_session_id.
func GenerateNumericSessionID() uint64 {
	timestamp := currentTimestampMillis()
	counter := globalCounter.Add(1) - 1
	return timestamp*1000 + counter
}

// invokeCounter provides unique platform IDs for invoke identifiers.
var invokeCounter atomic.Uint64

// NextInvokeCounter returns a monotonically increasing counter for invoke ID generation.
// Replaces Rust's `self as *const _ as usize` pattern.
func NextInvokeCounter() uint64 {
	return invokeCounter.Add(1)
}

// ResetForTesting resets internal counters (for testing purposes only).
//
// Ports Rust unique_id_generator::reset_for_testing.
func ResetForTesting() {
	globalCounter.Store(0)
}
