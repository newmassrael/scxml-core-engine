// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0106 #111: the `_event.data` of an error.* event the engine raises
// is the record SCE_MESH.md section 10.7.1 pins, errorName, reason and detail,
// in the bytes every engine writes; the Rust twin is
// event::tests::a_communication_code_names_its_event_and_its_detail_is_escaped.

package sce

import (
	"testing"

	errorreason "github.com/newmassrael/sce-go-runtime/generated/error_reason"
)

func TestAPlatformErrorCarriesItsCodeAsARecord(t *testing.T) {
	raised := NewPlatformError(7, errorreason.ErrorReasonAssignmentFailed, "<assign> to 'x' failed")
	want := `{"errorName":"execution","reason":"assignmentFailed","detail":"<assign> to 'x' failed"}`
	if raised.Metadata.Data != want {
		t.Fatalf("data = %s, want %s", raised.Metadata.Data, want)
	}
	if raised.Metadata.EventType != EventTypePlatform {
		t.Fatalf("an error the processor raised is a platform event, got %v", raised.Metadata.EventType)
	}
}

func TestACommunicationCodeNamesItsEventAndItsDetailIsEscaped(t *testing.T) {
	if got := ErrorName(errorreason.ErrorReasonExpressionFailed); got != "execution" {
		t.Fatalf("expressionFailed is raised as %q", got)
	}
	if got := ErrorName(errorreason.ErrorReasonPayloadRefused); got != "execution" {
		t.Fatalf("payloadRefused is raised as %q", got)
	}
	if got := ErrorName(errorreason.ErrorReasonTargetUnreachable); got != "communication" {
		t.Fatalf("targetUnreachable is raised as %q", got)
	}
	got := ErrorData(errorreason.ErrorReasonTargetUnreachable, "say \"hi\"\n")
	want := `{"errorName":"communication","reason":"targetUnreachable","detail":"say \"hi\"\n"}`
	if got != want {
		t.Fatalf("data = %s, want %s", got, want)
	}
}
