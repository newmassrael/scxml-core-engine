// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package sce

import (
	"fmt"
	"math"
	"strconv"
	"strings"
)

// W3C SCXML 6.2: Send action helpers (target validation, routing classification,
// delay parsing).
//
// Ports Rust send helpers from sce-rust-runtime/src/helpers/send.rs.

// IsInvalidTarget checks if target is invalid (starts with '!') (W3C SCXML 6.2).
//
// Ports Rust send::is_invalid_target.
func IsInvalidTarget(target string) bool {
	return target != "" && strings.HasPrefix(target, "!")
}

// IsInternalTarget checks if target uses the internal event queue (W3C SCXML C.1).
// Target #_internal routes events to the internal queue (high priority).
//
// Ports Rust send::is_internal_target.
func IsInternalTarget(target string) bool {
	return target == InternalTarget
}

// IsChildInvokeTarget checks if target is a child invoke session (W3C SCXML 6.4).
// Targets matching #_{invokeid} (but not #_parent, #_internal, or #_scxml_*)
// are child invoke targets.
//
// Ports Rust send::is_child_invoke_target.
func IsChildInvokeTarget(target string) bool {
	if !strings.HasPrefix(target, InvokeTargetPrefix) {
		return false
	}
	// Exclude special reserved targets
	if target == ParentTarget || target == InternalTarget {
		return false
	}
	// Exclude SCXML session targets
	if strings.HasPrefix(target, SCXMLSessionTargetPrefix) {
		return false
	}
	return true
}

// ExtractInvokeID extracts invoke ID from a child target (W3C SCXML 6.4).
// Given "#_{invokeid}", returns "{invokeid}".
//
// Ports Rust send::extract_invoke_id.
func ExtractInvokeID(target string) string {
	if strings.HasPrefix(target, InvokeTargetPrefix) {
		return target[len(InvokeTargetPrefix):]
	}
	return target
}

// IsHTTPTarget checks if target is an HTTP URL (W3C SCXML C.2).
//
// Ports Rust send::is_http_target.
func IsHTTPTarget(target string) bool {
	return strings.HasPrefix(target, "http://") || strings.HasPrefix(target, "https://")
}

// ValidateTarget validates a send target (W3C SCXML 6.2).
// Returns nil if valid, or an error if invalid (error.execution should be raised).
//
// Ports Rust send::validate_target.
func ValidateTarget(target string) error {
	if IsInvalidTarget(target) {
		return fmt.Errorf("invalid target value: %s", target)
	}
	return nil
}

// IsUnreachableTarget checks if target is unreachable (W3C SCXML C.1).
// Empty or "undefined" targets indicate unreachable sessions, requiring
// error.communication.
//
// Ports Rust send::is_unreachable_target.
func IsUnreachableTarget(target string) bool {
	return target == "" || target == "undefined"
}

// RequiresTargetAttribute checks if send type requires a target attribute
// (W3C SCXML C.2). BasicHTTP Event I/O Processor requires a target URL.
//
// Ports Rust send::requires_target_attribute.
func RequiresTargetAttribute(sendType string) bool {
	return sendType == BasicHTTPEventProcessorType
}

// IsSupportedSendType checks if send type is supported (W3C SCXML 6.2).
// Supported types: SCXML Event Processor (default), BasicHTTP Event Processor.
//
// Ports Rust send::is_supported_send_type.
func IsSupportedSendType(sendType string) bool {
	return sendType == "" ||
		sendType == SCXMLEventProcessorType ||
		sendType == BasicHTTPEventProcessorType
}

// ValidateBasicHTTPSend validates BasicHTTP send parameters (W3C SCXML C.2).
// Returns nil if valid, or an error if target is required but missing.
//
// Ports Rust send::validate_basic_http_send.
func ValidateBasicHTTPSend(sendType, target, targetExpr string) error {
	if RequiresTargetAttribute(sendType) && target == "" && targetExpr == "" {
		return fmt.Errorf("BasicHTTPEventProcessor requires target attribute")
	}
	return nil
}

// ParseDelayToMs parses a CSS2-style time duration into milliseconds
// (W3C SCXML 6.2).
//
// Accepts the same surface syntax as the Rust parse_delay_to_ms:
//   - "1s", "1.5s" -> seconds (converted to ms)
//   - "250ms" -> milliseconds
//   - bare number "500" -> milliseconds
//
// Returns (milliseconds, true) on success or (0, false) on unparseable input.
//
// Ports Rust send::parse_delay_to_ms.
func ParseDelayToMs(s string) (uint64, bool) {
	s = strings.TrimSpace(s)
	if s == "" {
		return 0, true
	}

	// Check suffix (order matters: "ms" before "s")
	if strings.HasSuffix(s, "ms") {
		num := strings.TrimSpace(s[:len(s)-2])
		f, err := strconv.ParseFloat(num, 64)
		if err != nil {
			return 0, false
		}
		return uint64(math.Max(0, f)), true
	}

	if strings.HasSuffix(s, "s") {
		num := strings.TrimSpace(s[:len(s)-1])
		f, err := strconv.ParseFloat(num, 64)
		if err != nil {
			return 0, false
		}
		return uint64(math.Max(0, f) * 1000), true
	}

	// Bare number: interpret as milliseconds (matches Rust fallback)
	f, err := strconv.ParseFloat(s, 64)
	if err != nil {
		return 0, false
	}
	return uint64(math.Max(0, f)), true
}
