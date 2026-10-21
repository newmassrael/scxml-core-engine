// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package optional

import (
	"errors"
	"testing"
)

// The zero value is absent, and each constructor builds what it names.
func TestTheZeroValueIsAbsentAndEachConstructorBuildsWhatItNames(t *testing.T) {
	var zero Of[int64]
	if zero.IsPresent() {
		t.Fatalf("the zero value is present")
	}
	if !zero.Equal(None[int64]()) {
		t.Fatalf("the zero value is not None")
	}
	v, ok := Some[int64](12).Get()
	if !ok || v != 12 {
		t.Fatalf("Some(12) reads %d, %v", v, ok)
	}
	if _, ok := None[string]().Get(); ok {
		t.Fatalf("None reads present")
	}
	if got := Some("soon").Proven(); got != "soon" {
		t.Fatalf("Proven reads %q", got)
	}
}

// Equality is total: two absent values are equal, an absent value equals no
// present one, and two present ones are equal by their values, a byte string by
// its bytes.
func TestEqualityIsTotal(t *testing.T) {
	cases := []struct {
		name string
		a, b Of[int64]
		want bool
	}{
		{"both absent", None[int64](), None[int64](), true},
		{"absent and present", None[int64](), Some[int64](0), false},
		{"present and absent", Some[int64](0), None[int64](), false},
		{"equal values", Some[int64](7), Some[int64](7), true},
		{"other values", Some[int64](7), Some[int64](8), false},
	}
	for _, c := range cases {
		if got := c.a.Equal(c.b); got != c.want {
			t.Errorf("%s: Equal is %v", c.name, got)
		}
	}
	if !Some([]byte("ab")).Equal(Some([]byte("ab"))) {
		t.Errorf("two byte strings of the same bytes are not equal")
	}
	if Some([]byte("ab")).Equal(Some([]byte("ac"))) {
		t.Errorf("two byte strings of other bytes are equal")
	}
}

// A default is evaluated only where the value is absent.
func TestADefaultIsEvaluatedOnlyWhereTheValueIsAbsent(t *testing.T) {
	calls := 0
	fallback := func() int64 { calls++; return -1 }
	if got := Otherwise(Some[int64](12), fallback); got != 12 || calls != 0 {
		t.Fatalf("a present value read %d with %d calls of the default", got, calls)
	}
	if got := Otherwise(None[int64](), fallback); got != -1 || calls != 1 {
		t.Fatalf("an absent value read %d with %d calls of the default", got, calls)
	}
	optionalCalls := 0
	optionalFallback := func() Of[int64] { optionalCalls++; return None[int64]() }
	if got := OtherwiseOptional(Some[int64](3), optionalFallback); !got.Equal(Some[int64](3)) || optionalCalls != 0 {
		t.Fatalf("a present value read %v with %d calls of the default", got, optionalCalls)
	}
	if got := OtherwiseOptional(None[int64](), optionalFallback); got.IsPresent() || optionalCalls != 1 {
		t.Fatalf("an absent value read %v with %d calls of the default", got, optionalCalls)
	}
}

// Map applies to a present value only, and Wire writes an absent value as nil.
func TestMapAndWireLeaveAnAbsentValueAbsent(t *testing.T) {
	calls := 0
	double := func(v int64) int64 { calls++; return 2 * v }
	if got := Map(Some[int64](4), double); !got.Equal(Some[int64](8)) {
		t.Fatalf("Map of 4 is %v", got)
	}
	if got := Map(None[int64](), double); got.IsPresent() || calls != 1 {
		t.Fatalf("Map of an absent value is %v after %d calls", got, calls)
	}
	text := func(v int64) any { return v + 1 }
	if got := Wire(None[int64](), text); got != nil {
		t.Fatalf("an absent value crosses as %v", got)
	}
	if got := Wire(Some[int64](1), text); got != int64(2) {
		t.Fatalf("a present value crosses as %v", got)
	}
	if got, want := None[int64]().String(), "null"; got != want {
		t.Fatalf("an absent value is shown as %q", got)
	}
	if got, want := Some[int64](7).String(), "7"; got != want {
		t.Fatalf("a present value is shown as %q", got)
	}
}

// A field the delivery carries as null is absent, one it carries a value of is
// read, and one it lacks is refused without reading it.
func TestAFieldIsReadAbsentPresentOrRefused(t *testing.T) {
	fields := map[string]any{"due": nil, "title": "soon"}
	reads := 0
	read := func(name string) func() (string, error) {
		return func() (string, error) {
			reads++
			if text, ok := fields[name].(string); ok {
				return text, nil
			}
			return "", errors.New("not a text")
		}
	}
	due, err := Lift(fields, "due", read("due"))
	if err != nil || due.IsPresent() || reads != 0 {
		t.Fatalf("a null field read %v, %v after %d reads", due, err, reads)
	}
	title, err := Lift(fields, "title", read("title"))
	if err != nil || !title.Equal(Some("soon")) {
		t.Fatalf("a carried field read %v, %v", title, err)
	}
	if _, err := Lift(fields, "status", read("status")); err == nil || reads != 1 {
		t.Fatalf("a missing key was not refused before it was read (%d reads)", reads)
	}
}
