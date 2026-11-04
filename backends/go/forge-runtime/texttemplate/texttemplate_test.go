// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package texttemplate

import "testing"

const (
	openMarker  = "<<<TEXT"
	closeMarker = "TEXT>>>"
)

// The cases every runtime's removal is held to (the Rust runtime's
// text_template tests): a value with no marker is kept whole; each marker is
// removed wherever it stands; a marker the removal rebuilds is removed too; and
// multi-byte text beside a marker is kept as it was.
func TestEveryMarkerIsRemovedUntilNoneIsLeft(t *testing.T) {
	cases := []struct{ value, kept string }{
		{"plain words", "plain words"},
		{"a<<<TEXTb", "ab"},
		{"aTEXT>>>b<<<TEXTc", "abc"},
		{"<<<TE<<<TEXTXT", ""},
		{"TEXT>>TEXT>>>>", ""},
		{"é<<<TEXTü", "éü"},
		{"", ""},
	}
	for _, c := range cases {
		if got := RemoveMarkers(c.value, openMarker, closeMarker); got != c.kept {
			t.Errorf("RemoveMarkers(%q) = %q, want %q", c.value, got, c.kept)
		}
	}
}

// Where both markers start at one byte, the longer is removed.
func TestTheLongerMarkerIsRemovedWhereBothStart(t *testing.T) {
	if got := RemoveMarkers("abcd", "ab", "abc"); got != "d" {
		t.Errorf("got %q, want %q", got, "d")
	}
}

// An empty marker removes nothing, and the other is still removed.
func TestAnEmptyMarkerIsNoMarker(t *testing.T) {
	if got := RemoveMarkers("x>>y", "", ">>"); got != "xy" {
		t.Errorf("got %q, want %q", got, "xy")
	}
	if got := RemoveMarkers("abc", "", ""); got != "abc" {
		t.Errorf("got %q, want %q", got, "abc")
	}
}
