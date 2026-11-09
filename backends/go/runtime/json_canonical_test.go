// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The JSON Canonicalization Scheme (RFC 8785) as WriteJCS writes it, held to
// the cases the Rust writer (`sce_rust_runtime::json::write_canonical`) and the
// Kotlin one (`com.sce.runtime.Json.writeJcs`) are held to, with the refusals
// only a Go string can call for.

package sce

import (
	"errors"
	"strings"
	"testing"
)

// writtenJCS is value in the scheme, failing the test where it is refused.
func writtenJCS(t *testing.T, value any) string {
	t.Helper()
	text, err := WriteJCS(value)
	if err != nil {
		t.Fatalf("a canonical value is written: %v", err)
	}
	return text
}

// refusedJCS is why value is refused, failing the test where it is written.
func refusedJCS(t *testing.T, value any) string {
	t.Helper()
	text, err := WriteJCS(value)
	var refusal *NotCanonical
	if !errors.As(err, &refusal) {
		t.Fatalf("%#v is refused, not written as %q (%v)", value, text, err)
	}
	return refusal.Error()
}

// codePoint is the text of the one character at the code point point: the
// characters of a case are named by their code points, so the source holds
// none of them.
func codePoint(point rune) string {
	return string(point)
}

// objectOf is the object of pairs, its members set in the order given.
func objectOf(pairs ...any) *JSONObject {
	object := NewJSONObject()
	for i := 0; i+1 < len(pairs); i += 2 {
		object.Set(pairs[i].(string), pairs[i+1])
	}
	return object
}

// RFC 8785 section 3.2.3, the example of the RFC: members are sorted by their
// names as UTF-16 code units, so U+1F600, whose first unit is 0xD83D, comes
// before U+FB33, which the order of Go's strings, by their UTF-8 bytes, would
// put after it; U+0080 is written as it is, since only U+0000 to U+001F are
// escaped.
func TestCanonicalMembersAreSortedByTheirUTF16CodeUnits(t *testing.T) {
	euro, dalet, emoji := codePoint(0x20AC), codePoint(0xFB33), codePoint(0x1F600)
	control, oWithDiaeresis := codePoint(0x0080), codePoint(0x00F6)
	value := objectOf(
		euro, "Euro Sign",
		"\r", "Carriage Return",
		dalet, "Hebrew Letter Dalet With Dagesh",
		"1", "One",
		emoji, "Emoji: Grinning Face",
		control, "Control",
		oWithDiaeresis, "Latin Small Letter O With Diaeresis",
	)
	want := `{"\r":"Carriage Return","1":"One","` + control + `":"Control","` +
		oWithDiaeresis + `":"Latin Small Letter O With Diaeresis","` + euro + `":"Euro Sign","` +
		emoji + `":"Emoji: Grinning Face","` +
		dalet + `":"Hebrew Letter Dalet With Dagesh"}`
	if got := writtenJCS(t, value); got != want {
		t.Fatalf("wrote %q, want %q", got, want)
	}
}

// RFC 8785 section 3.2.2.2: a text is escaped as `JSON.stringify` escapes it,
// the five short forms, `\u00xx` in lower-case hex for the other controls, and
// nothing else: not `/`, not U+007F, not a character past U+FFFF, and not the
// `<`, `>` and `&` that encoding/json escapes.
func TestACanonicalTextEscapesOnlyWhatJSONStringifyEscapes(t *testing.T) {
	emoji := codePoint(0x1F600)
	value := "\b\t\n\f\r\"\\\x00\x1f\x7f/<>&" + emoji
	want := `"\b\t\n\f\r\"\\\u0000\u001f` + "\x7f/<>&" + emoji + `"`
	if got := writtenJCS(t, value); got != want {
		t.Fatalf("wrote %q, want %q", got, want)
	}
}

// Nested arrays and objects are written with no whitespace, each object's
// members sorted at its own level, an array's items in their order.
func TestACanonicalValueHasNoWhitespaceAndSortsEveryLevel(t *testing.T) {
	value := objectOf(
		"b", []any{objectOf("z", nil, "a", false), "y", []any{}},
		"a", NewJSONObject(),
	)
	want := `{"a":{},"b":[{"a":false,"z":null},"y",[]]}`
	if got := writtenJCS(t, value); got != want {
		t.Fatalf("wrote %q, want %q", got, want)
	}
}

// RFC 8785 section 3.2.2.3: a number is written as ECMAScript writes the double
// it reads as; one past the range of a double is refused, and so is a spelling
// that is no JSON number, though strconv reads it.
func TestACanonicalNumberIsTheDoubleItReadsAs(t *testing.T) {
	for spelling, want := range map[string]string{
		"1E3":   "1000",
		"-0":    "0",
		"0.10":  "0.1",
		"1e21":  "1e+21",
		"1e-7":  "1e-7",
		"7":     "7",
		"-2.50": "-2.5",
	} {
		if got := writtenJCS(t, JSONNumber(spelling)); got != want {
			t.Errorf("%s: wrote %q, want %q", spelling, got, want)
		}
	}
	if why := refusedJCS(t, JSONNumber("1e400")); !strings.Contains(why, "1e400") {
		t.Errorf("the refusal names the number: %s", why)
	}
	for _, spelling := range []string{"0x10", "+1", "Inf", "01", "1."} {
		if why := refusedJCS(t, JSONNumber(spelling)); !strings.Contains(why, "is not a number") {
			t.Errorf("%s: %s", spelling, why)
		}
	}
}

// RFC 8785 section 3.2.2.2 writes a text as UTF-16 does: a Go string that is
// not UTF-8 has no such form, as a value or as a member's name, at whatever
// depth.
func TestATextThatIsNotUTF8HasNoCanonicalForm(t *testing.T) {
	if why := refusedJCS(t, []any{"a\xffb"}); !strings.Contains(why, "not UTF-8") {
		t.Errorf("a value: %s", why)
	}
	if why := refusedJCS(t, objectOf("ok", objectOf("\xff", true))); !strings.Contains(why, "not UTF-8") {
		t.Errorf("a name: %s", why)
	}
}

// A value of a type that is none of the six a JSON value is held as is refused
// by its type, at whatever depth.
func TestAValueOfAnotherTypeHasNoCanonicalForm(t *testing.T) {
	if why := refusedJCS(t, objectOf("n", []any{7})); !strings.Contains(why, "int is not a JSON value") {
		t.Errorf("an int: %s", why)
	}
}
