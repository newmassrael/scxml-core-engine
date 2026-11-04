// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Package texttemplate is the fence of a text template's untrusted hole
// (docs/adr/0087, section 3.1, item 4): the value is put between two markers,
// and every occurrence of either marker is first removed from it, so that the
// value cannot close the fence and write outside it.
//
// The removal repeats, taking the leftmost occurrence of either marker (the
// longer first where both start at one byte), until neither occurs: a value
// cannot rebuild a marker out of two halves around one that is removed. It ends
// because each pass shortens the value. The markers are printable ASCII, which
// no byte of a multi-byte UTF-8 sequence is, so removing them from the bytes of
// a text leaves a UTF-8 text. A Go string is those bytes.
//
// The package holds no state and does no I/O, as every forge runtime package.
package texttemplate

import "bytes"

// RemoveMarkers answers value with every occurrence of openMarker and
// closeMarker removed, by the rule of the package. An empty marker is no marker
// (a template's fence holds none; the build refuses it).
func RemoveMarkers(value, openMarker, closeMarker string) string {
	opening, closing := []byte(openMarker), []byte(closeMarker)
	longest := len(opening)
	if len(closing) > longest {
		longest = len(closing)
	}
	text := []byte(value)
	from := 0
	for {
		at, marker := -1, 0
		for i := from; i < len(text) && at < 0; i++ {
			rest := text[i:]
			opens := len(opening) > 0 && bytes.HasPrefix(rest, opening)
			closes := len(closing) > 0 && bytes.HasPrefix(rest, closing)
			switch {
			case opens && closes:
				at, marker = i, longest
			case opens:
				at, marker = i, len(opening)
			case closes:
				at, marker = i, len(closing)
			}
		}
		if at < 0 {
			return string(text)
		}
		text = append(text[:at], text[at+marker:]...)
		// Before at - (longest - 1) nothing changed and nothing matched, so a
		// marker the removal made can only start at or after it.
		from = at - (longest - 1)
		if from < 0 {
			from = 0
		}
	}
}
