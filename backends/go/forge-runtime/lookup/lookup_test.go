// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package lookup

import "testing"

// A key the table holds answers its value, and one it does not answers the
// zero value and false.
func TestLookupAnswersTheValueOrAMiss(t *testing.T) {
	keys := []uint8{1, 2, 3}
	values := []int32{100, 200, 0}
	for _, c := range []struct {
		needle uint8
		want   int32
		ok     bool
	}{
		{1, 100, true},
		{3, 0, true},
		{9, 0, false},
	} {
		got, ok := Lookup(keys, values, c.needle)
		if got != c.want || ok != c.ok {
			t.Errorf("Lookup(%d) = (%d, %v), want (%d, %v)", c.needle, got, ok, c.want, c.ok)
		}
	}
}

// Find answers a hit as a present value, including a value that is the zero
// of its type, and a miss as the absent value: the two are told apart by
// presence, never by the value.
func TestFindTellsAZeroValueFromAMiss(t *testing.T) {
	keys := []uint8{1, 2, 3}
	values := []int32{100, 200, 0}
	for _, c := range []struct {
		needle  uint8
		want    int32
		present bool
	}{
		{2, 200, true},
		{3, 0, true},
		{9, 0, false},
	} {
		o := Find(keys, values, c.needle)
		got, present := o.Get()
		if present != c.present || got != c.want || o.IsPresent() != c.present {
			t.Errorf("Find(%d) = (%d, %v), want (%d, %v)", c.needle, got, present, c.want, c.present)
		}
	}
}
