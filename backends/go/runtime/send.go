// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package sce

import (
	"fmt"
	"math"
	"strings"
)

// §scxml-6.2: Send action helpers (target validation, routing classification,
// delay parsing).
//
// Ports Rust send helpers from backends/rust/runtime/src/helpers/send.rs.

// IsInvalidTarget checks if target is invalid (starts with '!') (§scxml-6.2).
//
// Ports Rust send::is_invalid_target.
func IsInvalidTarget(target string) bool {
	return target != "" && strings.HasPrefix(target, "!")
}

// IsInternalTarget checks if target uses the internal event queue (§scxml-C-1).
// Target #_internal routes events to the internal queue (high priority).
//
// Ports Rust send::is_internal_target.
func IsInternalTarget(target string) bool {
	return target == InternalTarget
}

// IsChildInvokeTarget checks if target is a child invoke session (§scxml-6.4).
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

// ExtractInvokeID extracts invoke ID from a child target (§scxml-6.4).
// Given "#_{invokeid}", returns "{invokeid}".
//
// Ports Rust send::extract_invoke_id.
func ExtractInvokeID(target string) string {
	if strings.HasPrefix(target, InvokeTargetPrefix) {
		return target[len(InvokeTargetPrefix):]
	}
	return target
}

// IsHTTPTarget checks if target is an HTTP URL (§scxml-C-2).
//
// Ports Rust send::is_http_target.
func IsHTTPTarget(target string) bool {
	return strings.HasPrefix(target, "http://") || strings.HasPrefix(target, "https://")
}

// ValidateTarget validates a send target (§scxml-6.2).
// Returns nil if valid, or an error if invalid (error.execution should be raised).
//
// Ports Rust send::validate_target.
func ValidateTarget(target string) error {
	if IsInvalidTarget(target) {
		return fmt.Errorf("invalid target value: %s", target)
	}
	return nil
}

// IsUnreachableTarget checks if target is unreachable (§scxml-C-1).
// Empty or "undefined" targets indicate unreachable sessions, requiring
// error.communication.
//
// Ports Rust send::is_unreachable_target.
func IsUnreachableTarget(target string) bool {
	return target == "" || target == "undefined"
}

// MeshPeer returns the peer a <send target> names and true, when it names
// one: `#` followed by at least one character, where `#_` stays reserved for
// the targets §scxml-6.2.4 defines (`#_internal`, `#_parent`, ...).
//
// Ports Rust send::mesh_peer. Every copy of the predicate reads
// tests/mesh/mesh_target_cases.json, so a target one of them routes over Mesh
// is one they all do.
func MeshPeer(target string) (string, bool) {
	peer, ok := strings.CutPrefix(target, "#")
	if !ok || peer == "" || strings.HasPrefix(peer, "_") {
		return "", false
	}
	return peer, true
}

// IsMeshTarget reports whether a <send target> names a Mesh peer (see MeshPeer).
func IsMeshTarget(target string) bool {
	_, ok := MeshPeer(target)
	return ok
}

// RequiresTargetAttribute checks if send type requires a target attribute
// (§scxml-C-2). BasicHTTP Event I/O Processor requires a target URL.
//
// Ports Rust send::requires_target_attribute.
func RequiresTargetAttribute(sendType string) bool {
	return sendType == BasicHTTPEventProcessorType
}

// IsSupportedSendType checks if send type is supported (§scxml-6.2).
// Supported types: SCXML Event Processor (default), BasicHTTP Event Processor.
//
// Ports Rust send::is_supported_send_type.
func IsSupportedSendType(sendType string) bool {
	return sendType == "" ||
		sendType == SCXMLEventProcessorType ||
		sendType == BasicHTTPEventProcessorType
}

// ValidateBasicHTTPSend validates BasicHTTP send parameters (§scxml-C-2).
// Returns nil if valid, or an error if target is required but missing.
//
// Ports Rust send::validate_basic_http_send.
func ValidateBasicHTTPSend(sendType, target, targetExpr string) error {
	if RequiresTargetAttribute(sendType) && target == "" && targetExpr == "" {
		return fmt.Errorf("BasicHTTPEventProcessor requires target attribute")
	}
	return nil
}

// MaxDelayMs is the largest delay any engine can hold, in milliseconds: the
// positive range of a signed 64-bit count, because Kotlin's Long is signed and
// every engine answers the same text the same way.
const MaxDelayMs uint64 = math.MaxInt64

// ParseDelayToMs reads a <send> delay as the CSS2 time §scxml-6.2 names.
//
// The grammar is ARCHITECTURE.md's "Durations (Single Source of Truth)":
// surrounding ASCII whitespace aside, a non-negative number (digits with an
// optional fraction of at least one digit, or a leading "." and digits; no
// sign, no exponent) followed directly by "ms" or "s", either case. The
// milliseconds are computed in exact decimal and truncated, never through a
// float. tests/durations/css2_time.json holds the cases every engine is
// measured against.
//
// Returns (0, false) when the text is not a time — a bare number included —
// so the caller raises the argument error rather than choosing a wait.
func ParseDelayToMs(s string) (uint64, bool) {
	s = strings.Trim(s, " \t\n\r\f\v")
	var number string
	var scale uint64
	switch {
	case len(s) >= 2 && strings.EqualFold(s[len(s)-2:], "ms"):
		number, scale = s[:len(s)-2], 1
	case len(s) >= 1 && strings.EqualFold(s[len(s)-1:], "s"):
		number, scale = s[:len(s)-1], 1000
	default:
		return 0, false
	}
	whole, fraction, hasPoint := strings.Cut(number, ".")
	allDigits := func(part string) bool {
		for i := 0; i < len(part); i++ {
			if part[i] < '0' || part[i] > '9' {
				return false
			}
		}
		return true
	}
	// A number is digits, digits "." digits, or "." digits: the fraction is
	// never empty, and there is at least one digit somewhere.
	if !allDigits(whole) || (hasPoint && (fraction == "" || !allDigits(fraction))) {
		return 0, false
	}
	if whole == "" && !hasPoint {
		return 0, false
	}
	var ms uint64
	for i := 0; i < len(whole); i++ {
		digit := uint64(whole[i] - '0')
		if ms > (MaxDelayMs-digit)/10 {
			return 0, false
		}
		ms = ms*10 + digit
	}
	if ms > MaxDelayMs/scale {
		return 0, false
	}
	ms *= scale
	// Only the fraction digits that name whole milliseconds count; the rest
	// truncate.
	place := scale / 10
	for i := 0; i < len(fraction) && place > 0; i++ {
		add := uint64(fraction[i]-'0') * place
		if ms > MaxDelayMs-add {
			return 0, false
		}
		ms += add
		place /= 10
	}
	return ms, true
}
