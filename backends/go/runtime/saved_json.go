// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import (
	"fmt"
	"strings"
	"unicode/utf16"
	"unicode/utf8"
)

// The reader and writer a saved state's JSON text goes through.
//
// A saved state is one JSON document, the same on every backend, and what one
// backend wrote another reads (docs/adr/0045). That asks two things of the
// reader that encoding/json does not give: an object keeps the order its
// members were written in, since the text one machine writes is every
// backend's, and a number keeps the spelling it was written in, so a 64-bit
// value never passes through a float64. A value is a *JSONObject, a []any, a
// string, a bool, a JSONNumber or nil.
//
// Cross-language siblings: `com.sce.runtime.Json` (Kotlin) and
// `sce_rust_runtime::json` (Rust).

// JSONNumber is a number kept in the spelling it was written in, so no width is
// lost.
type JSONNumber string

// JSONObject is an object whose members are held in the order they were set.
type JSONObject struct {
	keys    []string
	members map[string]any
}

// NewJSONObject is an object with no members.
func NewJSONObject() *JSONObject {
	return &JSONObject{members: map[string]any{}}
}

// Set gives the member key the value, at the end when the object had none by
// that name and in place when it had.
func (o *JSONObject) Set(key string, value any) {
	if _, held := o.members[key]; !held {
		o.keys = append(o.keys, key)
	}
	o.members[key] = value
}

// Get is the member key, and whether the object has one.
func (o *JSONObject) Get(key string) (any, bool) {
	value, held := o.members[key]
	return value, held
}

// Has is whether the object has a member by the name key.
func (o *JSONObject) Has(key string) bool {
	_, held := o.members[key]
	return held
}

// Keys is the member names in the order they were set.
func (o *JSONObject) Keys() []string {
	return o.keys
}

// Len is the number of members.
func (o *JSONObject) Len() int {
	return len(o.keys)
}

// JSONError is why a text is not one JSON value.
type JSONError struct {
	message string
}

func (e *JSONError) Error() string { return e.message }

func jsonMalformed(what string) *JSONError {
	return &JSONError{message: "not JSON (" + what + ")"}
}

// ParseJSON reads text as exactly one JSON value, surrounding whitespace aside.
func ParseJSON(text string) (any, error) {
	reader := jsonReader{text: text}
	value, err := reader.readValue()
	if err != nil {
		return nil, err
	}
	reader.skipWhitespace()
	if !reader.atEnd() {
		return nil, &JSONError{message: "more than one JSON value"}
	}
	return value, nil
}

type jsonReader struct {
	text string
	at   int
}

func (r *jsonReader) atEnd() bool { return r.at >= len(r.text) }

func (r *jsonReader) skipWhitespace() {
	for r.at < len(r.text) {
		switch r.text[r.at] {
		case ' ', '\t', '\n', '\r':
			r.at++
		default:
			return
		}
	}
}

func (r *jsonReader) readValue() (any, error) {
	r.skipWhitespace()
	if r.atEnd() {
		return nil, jsonMalformed("it ends where a value was expected")
	}
	switch c := r.text[r.at]; {
	case c == '{':
		return r.readObject()
	case c == '[':
		return r.readArray()
	case c == '"':
		return r.readString()
	case c == 't':
		return r.readKeyword("true", true)
	case c == 'f':
		return r.readKeyword("false", false)
	case c == 'n':
		return r.readKeyword("null", nil)
	case c == '-' || (c >= '0' && c <= '9'):
		return r.readNumber()
	default:
		return nil, jsonMalformed(fmt.Sprintf("unexpected '%c'", c))
	}
}

func (r *jsonReader) readObject() (any, error) {
	r.at++ // '{'
	out := NewJSONObject()
	r.skipWhitespace()
	if !r.atEnd() && r.text[r.at] == '}' {
		r.at++
		return out, nil
	}
	for {
		r.skipWhitespace()
		if r.atEnd() || r.text[r.at] != '"' {
			return nil, jsonMalformed("a field name was expected")
		}
		key, err := r.readString()
		if err != nil {
			return nil, err
		}
		// An object whose names repeat has no single meaning: a reader that
		// takes the first and one that takes the last would read two values
		// from one text, so it is refused as the other backends refuse it.
		if out.Has(key) {
			return nil, jsonMalformed(fmt.Sprintf("the field \"%s\" appears twice", key))
		}
		r.skipWhitespace()
		if r.atEnd() || r.text[r.at] != ':' {
			return nil, jsonMalformed("a ':' was expected")
		}
		r.at++
		value, err := r.readValue()
		if err != nil {
			return nil, err
		}
		out.Set(key, value)
		r.skipWhitespace()
		if r.atEnd() {
			return nil, jsonMalformed("it ends unclosed")
		}
		switch r.text[r.at] {
		case ',':
			r.at++
		case '}':
			r.at++
			return out, nil
		default:
			return nil, jsonMalformed("a ',' or '}' was expected")
		}
	}
}

func (r *jsonReader) readArray() (any, error) {
	r.at++ // '['
	out := []any{}
	r.skipWhitespace()
	if !r.atEnd() && r.text[r.at] == ']' {
		r.at++
		return out, nil
	}
	for {
		value, err := r.readValue()
		if err != nil {
			return nil, err
		}
		out = append(out, value)
		r.skipWhitespace()
		if r.atEnd() {
			return nil, jsonMalformed("it ends unclosed")
		}
		switch r.text[r.at] {
		case ',':
			r.at++
		case ']':
			r.at++
			return out, nil
		default:
			return nil, jsonMalformed("a ',' or ']' was expected")
		}
	}
}

func (r *jsonReader) readString() (string, error) {
	r.at++ // '"'
	var sb strings.Builder
	for {
		if r.atEnd() {
			return "", jsonMalformed("a text ends unclosed")
		}
		c := r.text[r.at]
		r.at++
		switch c {
		case '"':
			return sb.String(), nil
		case '\\':
			if r.atEnd() {
				return "", jsonMalformed("an escape ends the text")
			}
			escape := r.text[r.at]
			r.at++
			switch escape {
			case '"':
				sb.WriteByte('"')
			case '\\':
				sb.WriteByte('\\')
			case '/':
				sb.WriteByte('/')
			case 'b':
				sb.WriteByte('\b')
			case 'f':
				sb.WriteByte('\f')
			case 'n':
				sb.WriteByte('\n')
			case 'r':
				sb.WriteByte('\r')
			case 't':
				sb.WriteByte('\t')
			case 'u':
				unit, err := r.readUnit()
				if err != nil {
					return "", err
				}
				// A high surrogate followed by a `\u` low one is one character, as
				// it is where a text is held as UTF-16; one on its own is no
				// character, and reads as U+FFFD as the Rust reader reads it.
				if utf16.IsSurrogate(rune(unit)) {
					if low, ok := r.readLowSurrogate(unit); ok {
						sb.WriteRune(utf16.DecodeRune(rune(unit), rune(low)))
						continue
					}
					sb.WriteRune(utf8.RuneError)
					continue
				}
				sb.WriteRune(rune(unit))
			default:
				return "", jsonMalformed(fmt.Sprintf("unknown escape '\\%c'", escape))
			}
		default:
			sb.WriteByte(c)
		}
	}
}

// readUnit reads the four hex digits of a `\u` escape.
func (r *jsonReader) readUnit() (uint16, error) {
	if r.at+4 > len(r.text) {
		return 0, jsonMalformed("a \\u escape is short")
	}
	var unit uint16
	for _, digit := range []byte(r.text[r.at : r.at+4]) {
		var value byte
		switch {
		case digit >= '0' && digit <= '9':
			value = digit - '0'
		case digit >= 'a' && digit <= 'f':
			value = digit - 'a' + 10
		case digit >= 'A' && digit <= 'F':
			value = digit - 'A' + 10
		default:
			return 0, jsonMalformed("a \\u escape is not hex")
		}
		unit = unit<<4 | uint16(value)
	}
	r.at += 4
	return unit, nil
}

// readLowSurrogate reads the `\u` low surrogate that completes the high one
// `high`, and consumes it only when it does.
func (r *jsonReader) readLowSurrogate(high uint16) (uint16, bool) {
	if high >= 0xDC00 || !strings.HasPrefix(r.text[r.at:], "\\u") {
		return 0, false
	}
	saved := r.at
	r.at += 2
	low, err := r.readUnit()
	if err != nil || low < 0xDC00 || low > 0xDFFF {
		r.at = saved
		return 0, false
	}
	return low, true
}

func (r *jsonReader) readNumber() (any, error) {
	start := r.at
	digits := func() {
		for !r.atEnd() && r.text[r.at] >= '0' && r.text[r.at] <= '9' {
			r.at++
		}
	}
	if !r.atEnd() && r.text[r.at] == '-' {
		r.at++
	}
	digits()
	if !r.atEnd() && r.text[r.at] == '.' {
		r.at++
		digits()
	}
	if !r.atEnd() && (r.text[r.at] == 'e' || r.text[r.at] == 'E') {
		r.at++
		if !r.atEnd() && (r.text[r.at] == '+' || r.text[r.at] == '-') {
			r.at++
		}
		digits()
	}
	slice := r.text[start:r.at]
	if slice == "" || slice == "-" {
		return nil, jsonMalformed("a number has no digits")
	}
	return JSONNumber(slice), nil
}

func (r *jsonReader) readKeyword(word string, value any) (any, error) {
	if !strings.HasPrefix(r.text[r.at:], word) {
		return nil, jsonMalformed("expected '" + word + "'")
	}
	r.at += len(word)
	return value, nil
}

// WriteJSON is value as compact JSON text. A number is written in the spelling
// it carries, so a value ParseJSON read is written back unchanged, and a text in
// the one form every engine writes ([EscapeJSONString]). A value of any type
// but the six a JSON value is held as is a programming error and panics.
func WriteJSON(value any) string {
	var out strings.Builder
	writeJSONInto(value, &out)
	return out.String()
}

func writeJSONInto(value any, out *strings.Builder) {
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
		out.WriteString(string(v))
	case string:
		out.WriteByte('"')
		out.WriteString(EscapeJSONString(v))
		out.WriteByte('"')
	case []any:
		out.WriteByte('[')
		for i, item := range v {
			if i > 0 {
				out.WriteByte(',')
			}
			writeJSONInto(item, out)
		}
		out.WriteByte(']')
	case *JSONObject:
		out.WriteByte('{')
		for i, key := range v.keys {
			if i > 0 {
				out.WriteByte(',')
			}
			out.WriteByte('"')
			out.WriteString(EscapeJSONString(key))
			out.WriteString(`":`)
			writeJSONInto(v.members[key], out)
		}
		out.WriteByte('}')
	default:
		panic(fmt.Sprintf("%T is not a JSON value; wrap a number in JSONNumber", value))
	}
}
