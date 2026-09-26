// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A typed request's start-site check and its adapter's reading are one rule:
// every value the check accepts is spelled as text its field's type parses
// back to the same value, and a value the field cannot hold is refused rather
// than narrowed (SCE Accepted Subset §2.12).

package sce

import (
	"math"
	"testing"
)

func requestOf(text string) HostInvokeRequest {
	return HostInvokeRequest{InvokeID: "perm", Params: map[string][]string{"f": {text}}}
}

func wire(t *testing.T, value any, ty RequestFieldType) string {
	t.Helper()
	text, err := RequestFieldWire(value, "f", ty)
	if err != nil {
		t.Fatalf("%v as %v was refused: %v", value, ty, err)
	}
	return text
}

func TestARequestFieldIsCheckedAndReadBackByOneRule(t *testing.T) {
	if got := RequestUnsigned[uint8](requestOf(wire(t, int64(255), RequestFieldUint8)), "f"); got != 255 {
		t.Errorf("uint8 255 read back as %d", got)
	}
	if got := RequestSigned[int16](requestOf(wire(t, -3.0, RequestFieldInt16)), "f"); got != -3 {
		t.Errorf("int16 -3 read back as %d", got)
	}
	if got := RequestFloat[float64](requestOf(wire(t, 0.1, RequestFieldFloat64)), "f"); got != 0.1 {
		t.Errorf("float64 0.1 read back as %v", got)
	}
	if got := RequestFloat[float32](requestOf(wire(t, 0.1, RequestFieldFloat32)), "f"); got != float32(0.1) {
		t.Errorf("float32 0.1 read back as %v", got)
	}
	if got := RequestBool(requestOf(wire(t, false, RequestFieldBool)), "f"); got {
		t.Errorf("bool false read back as true")
	}
	if got := RequestString(requestOf(wire(t, "a b", RequestFieldString)), "f"); got != "a b" {
		t.Errorf("text read back as %q", got)
	}

	for _, c := range []struct {
		value any
		ty    RequestFieldType
		why   string
	}{
		{int64(256), RequestFieldUint8, "past the width"},
		{int64(-1), RequestFieldUint32, "below zero"},
		{1.5, RequestFieldInt32, "not whole"},
		{math.NaN(), RequestFieldFloat64, "not finite"},
		{1e39, RequestFieldFloat32, "past float32"},
		{"2", RequestFieldUint8, "a text"},
		{int64(1), RequestFieldBool, "a number"},
		{int64(1), RequestFieldString, "a number"},
		{"abc", RequestFieldBytes(2), "past cap"},
		{"Ā", RequestFieldBytes(8), "no byte"},
	} {
		if _, err := RequestFieldWire(c.value, "f", c.ty); err == nil {
			t.Errorf("%s: %v as %v was accepted", c.why, c.value, c.ty)
		}
	}
}
