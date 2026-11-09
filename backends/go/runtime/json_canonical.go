// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import (
	"fmt"
	"math"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"unicode/utf16"
	"unicode/utf8"
)

// The JSON Canonicalization Scheme (RFC 8785): the one text of a value that
// two writers agree on byte for byte, which the test log of the test profile
// is written in (docs/SCE_TEST_PROFILE.md §12.5).
//
// Cross-language siblings: `sce_rust_runtime::json::write_canonical` (Rust),
// `com.sce.runtime.Json.writeJcs` (Kotlin), `SCE::Saved::writeCanonical`
// (C++).

// NotCanonical is why a value has no form in the JSON Canonicalization Scheme.
type NotCanonical struct {
	reason string
}

func (e *NotCanonical) Error() string { return e.reason }

// jsonNumberGrammar is the grammar of a JSON number (RFC 8259 section 6),
// which strconv.ParseFloat is wider than: it reads a hexadecimal mantissa, a
// leading `+`, `Inf` and `NaN` as well.
var jsonNumberGrammar = regexp.MustCompile(`^-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?$`)

// WriteJCS is value in the JSON Canonicalization Scheme (RFC 8785): no
// whitespace, the members of each object sorted by their names as sequences of
// UTF-16 code units (section 3.2.3), each text in the one form every engine
// writes (EscapeJSONString, which is ECMAScript's `JSON.stringify` of a text:
// section 3.2.2.2), and each number as ECMAScript spells the double it reads as
// (NumberText, section 3.2.2.3). Two values equal under the scheme are then
// written as the same bytes, whatever order their members were set in.
//
// It is not WriteJSON, which keeps the order the members were set in and the
// spelling a number carries. Nor is the order of the names Go's order of
// strings, which is the order of their UTF-8 bytes and so of their code points:
// the two differ where a name holds a character above U+FFFF, whose first
// UTF-16 unit, a surrogate, sorts below U+E000 to U+FFFF.
//
// A *JSONObject names each member once, which the scheme requires (section
// 3.1). Refused, with a *NotCanonical naming why: a JSONNumber that is no JSON
// number or is past the range of a double, which the scheme has no spelling
// for; a text or a name that is not UTF-8, which has no UTF-16 form to sort by
// or to write; and a value of a type that is none of the six a JSON value is
// held as.
func WriteJCS(value any) (string, error) {
	var out strings.Builder
	if err := writeJCSInto(value, &out); err != nil {
		return "", err
	}
	return out.String(), nil
}

// jcsName is a member's name and its UTF-16 code units, which it is sorted by.
type jcsName struct {
	name  string
	units []uint16
}

func writeJCSInto(value any, out *strings.Builder) error {
	switch v := value.(type) {
	case nil:
		out.WriteString("null")
	case bool:
		if v {
			out.WriteString("true")
		} else {
			out.WriteString("false")
		}
	case JSONNumber:
		spelling := string(v)
		if !jsonNumberGrammar.MatchString(spelling) {
			return &NotCanonical{reason: "'" + spelling + "' is not a number"}
		}
		// The grammar holds, so the only error left is a magnitude past the
		// largest double, which ParseFloat answers as an infinity.
		number, err := strconv.ParseFloat(spelling, 64)
		if err != nil || math.IsInf(number, 0) {
			return &NotCanonical{reason: "the number " + spelling +
				" is past the range of a double, which RFC 8785 writes every number as"}
		}
		out.WriteString(NumberText(number))
	case string:
		if err := writeJCSText(v, out); err != nil {
			return err
		}
	case []any:
		out.WriteByte('[')
		for i, item := range v {
			if i > 0 {
				out.WriteByte(',')
			}
			if err := writeJCSInto(item, out); err != nil {
				return err
			}
		}
		out.WriteByte(']')
	case *JSONObject:
		names := make([]jcsName, 0, len(v.keys))
		for _, key := range v.keys {
			if !utf8.ValidString(key) {
				return notUTF8(key)
			}
			names = append(names, jcsName{name: key, units: utf16.Encode([]rune(key))})
		}
		sort.Slice(names, func(i, j int) bool { return unitsBefore(names[i].units, names[j].units) })
		out.WriteByte('{')
		for i, member := range names {
			if i > 0 {
				out.WriteByte(',')
			}
			if err := writeJCSText(member.name, out); err != nil {
				return err
			}
			out.WriteByte(':')
			if err := writeJCSInto(v.members[member.name], out); err != nil {
				return err
			}
		}
		out.WriteByte('}')
	default:
		return &NotCanonical{reason: fmt.Sprintf("%T is not a JSON value; wrap a number in JSONNumber", value)}
	}
	return nil
}

// writeJCSText writes text quoted and escaped, or refuses it when it is not
// UTF-8.
func writeJCSText(text string, out *strings.Builder) error {
	if !utf8.ValidString(text) {
		return notUTF8(text)
	}
	out.WriteByte('"')
	out.WriteString(EscapeJSONString(text))
	out.WriteByte('"')
	return nil
}

func notUTF8(text string) *NotCanonical {
	return &NotCanonical{reason: fmt.Sprintf("the text %q is not UTF-8, which RFC 8785 writes every text as", text)}
}

// unitsBefore reports whether a sorts before b, unit by unit, a prefix first.
func unitsBefore(a, b []uint16) bool {
	for i := 0; i < len(a) && i < len(b); i++ {
		if a[i] != b[i] {
			return a[i] < b[i]
		}
	}
	return len(a) < len(b)
}
