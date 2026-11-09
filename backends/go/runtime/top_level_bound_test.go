// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A payload's own text and byte-string fields are held to their bound where the
// delivery is read, as a text and a byte string are at any depth
// (docs/adr/0086, section 4.4): PayloadTextWithin and PayloadBytes are the readers a
// generated lift of a schema of entries calls, and each refuses one past the
// bound with the code, pointer and bound of PayloadValue's own refusal.

package sce

import (
	"errors"
	"testing"
)

// refusedTooLong is whether err is a TooLong refusal at pointer naming bound.
func refusedTooLong(t *testing.T, err error, pointer string, bound uint32) {
	t.Helper()
	var refusal *PayloadRefusal
	if !errors.As(err, &refusal) {
		t.Fatalf("err = %v; want a *PayloadRefusal", err)
	}
	if refusal.Code() != RefusalTooLong {
		t.Errorf("code = %v; want tooLong", refusal.Code().Name())
	}
	if refusal.Pointer() != pointer {
		t.Errorf("pointer = %q; want %q", refusal.Pointer(), pointer)
	}
	if got, ok := refusal.Bound(); !ok || got != bound {
		t.Errorf("bound = %d (%t); want %d", got, ok, bound)
	}
}

func TestATopLevelTextIsHeldToItsBoundWhereItIsRead(t *testing.T) {
	fields, err := LiftPayload(`{"note": "abcde", "wide": "éé"}`)
	if err != nil {
		t.Fatalf("the object did not read: %v", err)
	}
	_, err = PayloadTextWithin(fields, "note", 4)
	refusedTooLong(t, err, "/note", 4)
	if err != nil && err.Error() != "'note' is 5 bytes, past the 4 its type declares" {
		t.Errorf("reason = %q", err.Error())
	}
	if note, err := PayloadTextWithin(fields, "note", 5); err != nil || note != "abcde" {
		t.Errorf("note = %q, %v; want the five bytes it carried", note, err)
	}
	// The bound counts UTF-8 bytes: two characters, four bytes.
	_, err = PayloadTextWithin(fields, "wide", 3)
	refusedTooLong(t, err, "/wide", 3)
	if wide, err := PayloadTextWithin(fields, "wide", 4); err != nil || wide != "éé" {
		t.Errorf("wide = %q, %v; want the four bytes it carried", wide, err)
	}
}

func TestATopLevelByteStringIsHeldToItsBoundWhereItIsRead(t *testing.T) {
	fields, err := LiftPayload(`{"blob": "abcde"}`)
	if err != nil {
		t.Fatalf("the object did not read: %v", err)
	}
	_, err = PayloadBytes(fields, "blob", 4)
	refusedTooLong(t, err, "/blob", 4)
	if blob, err := PayloadBytes(fields, "blob", 5); err != nil || string(blob) != "abcde" {
		t.Errorf("blob = %q, %v; want the five bytes it carried", blob, err)
	}
}
