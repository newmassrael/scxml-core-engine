// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A union is saved as one object, its tag beside the members of its case's
// record, and read back by its tag first (docs/adr/0092, section 6.1 item 7).

package sce

import (
	"strings"
	"testing"
)

// A case that holds a record writes its tag first and then the record's members
// in the order the record wrote them; an empty case writes its tag alone.
func TestAUnionIsSavedAsItsTagBesideTheMembersOfItsCase(t *testing.T) {
	record := NewJSONObject()
	record.Set("width", SavedWriteUint16(3))
	record.Set("height", SavedWriteUint16(4))
	for _, c := range []struct {
		saved any
		want  string
	}{
		{SavedWriteUnion("kind", "rect", record), `{"kind":"rect","width":3,"height":4}`},
		{SavedWriteUnion("kind", "dot", nil), `{"kind":"dot"}`},
	} {
		if got := WriteJSON(c.saved); got != c.want {
			t.Errorf("a union is saved as %s, want %s", got, c.want)
		}
	}
}

// The tag is read before any field: a saved union's case is named by the text
// under its key, and a value that is no object, one with no tag and one whose tag
// is not a text are refused naming the place.
func TestASavedUnionIsReadByItsTagFirst(t *testing.T) {
	saved, err := ParseJSON(`{"width":3,"kind":"rect","height":4}`)
	if err != nil {
		t.Fatal(err)
	}
	if got, err := SavedReadUnionTag(saved, "built", "kind"); err != nil || got != "rect" {
		t.Errorf("the tag read %q, %v; want rect", got, err)
	}
	for _, c := range []struct {
		text, want string
	}{
		{`[1]`, "'built' is not an object"},
		{`{"width":3}`, "'built' has no field 'kind'"},
		{`{"kind":3}`, "'built.kind' is not a text"},
		{`{"kind":null}`, "'built.kind' is absent (null)"},
	} {
		value, err := ParseJSON(c.text)
		if err != nil {
			t.Fatal(err)
		}
		_, err = SavedReadUnionTag(value, "built", "kind")
		if err == nil || !strings.Contains(err.Error(), c.want) {
			t.Errorf("%s: refused %v, want one naming %q", c.text, err, c.want)
		}
	}
}

// A list of a type that states a lower count is read back only between its two
// counts, both held before any element is read; a list that states none reads
// empty, as it did.
func TestASavedListIsHeldToTheLowerCountItsTypeStates(t *testing.T) {
	read := func(text string, least int) ([]uint8, error) {
		value, err := ParseJSON(text)
		if err != nil {
			t.Fatal(err)
		}
		return SavedReadListWithin(value, "points", least, 3, SavedReadUint8)
	}
	if got, err := read(`[1,2]`, 1); err != nil || len(got) != 2 || got[0] != 1 || got[1] != 2 {
		t.Errorf("a list within its counts read %v, %v", got, err)
	}
	if got, err := read(`[]`, 0); err != nil || len(got) != 0 {
		t.Errorf("an empty list of no lower count read %v, %v", got, err)
	}
	for _, c := range []struct {
		text  string
		least int
		want  string
	}{
		{`[]`, 1, "'points' holds 0 elements, fewer than the 1 its type declares"},
		{`[1,2,3,4]`, 1, "'points' holds 4 elements, past the 3 it is bounded by"},
		{`["x"]`, 2, "fewer than the 2"},
	} {
		_, err := read(c.text, c.least)
		if err == nil || !strings.Contains(err.Error(), c.want) {
			t.Errorf("%s: refused %v, want one naming %q", c.text, err, c.want)
		}
	}
}
