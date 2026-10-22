// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import (
	"fmt"
	"math"
	"strconv"
)

// How each `sce:type` is written into a saved state and read back: only if it is
// one, since a value of another kind, or one its width cannot hold, is refused
// rather than converted. The functions a generated machine's `Save` and
// `Restore` call, one per type, named as the type is.
//
// A value is written as the JSON value model holds it (a JSONNumber, a string, a
// bool, a []any, a *JSONObject, nil) and read back from it, a refusal naming
// what holds a value that is not the type's.
//
// Cross-language siblings: `com.sce.runtime.SavedValues` (Kotlin) and the
// `SavedValue` trait of `sce_rust_runtime::saved_state` (Rust).

// notA is the refusal of value, saved for what, which is not kind (`a number`,
// `a text`, ...). A nil is named apart: it is an absent value (docs/adr/0055),
// which only a place the restoring build declares optional reads, so a state
// saved after a widening is refused at its first null by a build from before it.
func notA(value any, what, kind string) error {
	if value == nil {
		return refuse("'%s' is absent (null), and its type is not optional", what)
	}
	return refuse("'%s' is not %s", what, kind)
}

func savedNumber(value any, what string) (string, error) {
	number, ok := value.(JSONNumber)
	if !ok {
		return "", notA(value, what, "a number")
	}
	return string(number), nil
}

// savedWhole is a number of at least min and at most max, written as a whole.
func savedWhole(value any, what string, min, max int64) (int64, error) {
	text, err := savedNumber(value, what)
	if err != nil {
		return 0, err
	}
	n, parseErr := strconv.ParseInt(text, 10, 64)
	if parseErr != nil || n < min || n > max {
		return 0, refuse("'%s' (%s) is not a whole number its type can hold", what, text)
	}
	return n, nil
}

// SavedWriteUint8 and the rest of the Write functions give a value its saved
// form. A 64-bit integer is written as a text: a reader that holds JSON numbers
// as doubles would lose its low bits, and a saved state outlives the backend
// that wrote it.
func SavedWriteUint8(value uint8) any   { return JSONNumber(strconv.FormatUint(uint64(value), 10)) }
func SavedWriteUint16(value uint16) any { return JSONNumber(strconv.FormatUint(uint64(value), 10)) }
func SavedWriteUint32(value uint32) any { return JSONNumber(strconv.FormatUint(uint64(value), 10)) }
func SavedWriteInt8(value int8) any     { return JSONNumber(strconv.FormatInt(int64(value), 10)) }
func SavedWriteInt16(value int16) any   { return JSONNumber(strconv.FormatInt(int64(value), 10)) }
func SavedWriteInt32(value int32) any   { return JSONNumber(strconv.FormatInt(int64(value), 10)) }
func SavedWriteUint64(value uint64) any { return strconv.FormatUint(value, 10) }
func SavedWriteInt64(value int64) any   { return strconv.FormatInt(value, 10) }

// SavedWriteFloat64 gives a real its saved form: the one number text every engine
// writes (ARCHITECTURE.md, "JSON Number Text"), and, for a value that is not
// finite, which JSON has no spelling for, the text every backend reads it back
// from.
func SavedWriteFloat64(value float64) any {
	switch {
	case math.IsNaN(value):
		return "NaN"
	case math.IsInf(value, 1):
		return "Infinity"
	case math.IsInf(value, -1):
		return "-Infinity"
	}
	return JSONNumber(NumberText(value))
}

// SavedWriteFloat32 gives a 32-bit real the saved form of the 64-bit real it
// widens to, which is exact: the number a reader that holds JSON numbers as
// doubles finds is the number the machine holds, `0.30000001192092896` and not
// the shorter `0.3` a single prints for it, which is another number to that
// reader.
func SavedWriteFloat32(value float32) any { return SavedWriteFloat64(float64(value)) }

func SavedWriteBool(value bool) any     { return value }
func SavedWriteString(value string) any { return value }

// SavedWriteBytes gives a byte string its byte-exact Latin-1 text, each byte the
// character of that code point (docs/adr/0005, decision 2): the spelling it
// takes on the wire, so one value is one text on every backend, and a JSON
// reader that has no byte string reads it as a string.
func SavedWriteBytes(value []byte) any { return BytesAsPayloadText(value) }

// SavedWriteList writes a list, each element by write.
func SavedWriteList[T any](value []T, write func(T) any) any {
	items := make([]any, len(value))
	for at, item := range value {
		items[at] = write(item)
	}
	return items
}

func SavedReadUint8(value any, what string) (uint8, error) {
	n, err := savedWhole(value, what, 0, math.MaxUint8)
	return uint8(n), err
}

func SavedReadUint16(value any, what string) (uint16, error) {
	n, err := savedWhole(value, what, 0, math.MaxUint16)
	return uint16(n), err
}

func SavedReadUint32(value any, what string) (uint32, error) {
	n, err := savedWhole(value, what, 0, math.MaxUint32)
	return uint32(n), err
}

func SavedReadInt8(value any, what string) (int8, error) {
	n, err := savedWhole(value, what, math.MinInt8, math.MaxInt8)
	return int8(n), err
}

func SavedReadInt16(value any, what string) (int16, error) {
	n, err := savedWhole(value, what, math.MinInt16, math.MaxInt16)
	return int16(n), err
}

func SavedReadInt32(value any, what string) (int32, error) {
	n, err := savedWhole(value, what, math.MinInt32, math.MaxInt32)
	return int32(n), err
}

func savedWideText(value any, what string) (string, error) {
	text, ok := value.(string)
	if !ok {
		return "", notA(value, what, "a 64-bit whole number written as a text")
	}
	return text, nil
}

func SavedReadUint64(value any, what string) (uint64, error) {
	text, err := savedWideText(value, what)
	if err != nil {
		return 0, err
	}
	n, parseErr := strconv.ParseUint(text, 10, 64)
	if parseErr != nil {
		return 0, refuse("'%s' (%s) is not a whole number its type can hold", what, text)
	}
	return n, nil
}

func SavedReadInt64(value any, what string) (int64, error) {
	text, err := savedWideText(value, what)
	if err != nil {
		return 0, err
	}
	n, parseErr := strconv.ParseInt(text, 10, 64)
	if parseErr != nil {
		return 0, refuse("'%s' (%s) is not a whole number its type can hold", what, text)
	}
	return n, nil
}

func savedReal(value any, what string) (float64, error) {
	switch value {
	case "NaN":
		return math.NaN(), nil
	case "Infinity":
		return math.Inf(1), nil
	case "-Infinity":
		return math.Inf(-1), nil
	}
	text, err := savedNumber(value, what)
	if err != nil {
		return 0, err
	}
	// A number past the range of a 64-bit real reads as the infinity it rounds
	// to, as every other backend reads it; only a text that is no number is
	// refused.
	real, parseErr := strconv.ParseFloat(text, 64)
	if parseErr != nil {
		if numErr, ok := parseErr.(*strconv.NumError); !ok || numErr.Err != strconv.ErrRange {
			return 0, refuse("'%s' (%s) is not a number", what, text)
		}
	}
	return real, nil
}

func SavedReadFloat32(value any, what string) (float32, error) {
	real, err := savedReal(value, what)
	return float32(real), err
}

func SavedReadFloat64(value any, what string) (float64, error) {
	return savedReal(value, what)
}

func SavedReadBool(value any, what string) (bool, error) {
	truth, ok := value.(bool)
	if !ok {
		return false, notA(value, what, "a truth value")
	}
	return truth, nil
}

func SavedReadString(value any, what string) (string, error) {
	text, ok := value.(string)
	if !ok {
		return "", notA(value, what, "a text")
	}
	return text, nil
}

// SavedReadBoundedString is a text read back only if it holds no more than the
// capacity UTF-8 bytes the machine bounds it by: a machine never holds more, and
// a restored one must not be the first to.
func SavedReadBoundedString(value any, what string, capacity int) (string, error) {
	text, err := SavedReadString(value, what)
	if err != nil {
		return "", err
	}
	if len(text) > capacity {
		return "", refuse("'%s' holds %d UTF-8 bytes, past the %d it is bounded by", what, len(text), capacity)
	}
	return text, nil
}

// SavedReadBytes is a byte string read back from its Latin-1 text
// ([SavedWriteBytes]), only if it holds no more than the capacity bytes the
// machine bounds it by. A character past U+00FF is no byte, and is refused
// rather than cut.
func SavedReadBytes(value any, what string, capacity int) ([]byte, error) {
	text, err := SavedReadString(value, what)
	if err != nil {
		return nil, err
	}
	bytes := make([]byte, 0, len(text))
	for _, point := range text {
		if point > 0xFF {
			return nil, refuse("'%s' holds U+%04X, which is past U+00FF and so no byte", what, point)
		}
		bytes = append(bytes, byte(point))
	}
	if len(bytes) > capacity {
		return nil, refuse("'%s' holds %d bytes, past the %d it is bounded by", what, len(bytes), capacity)
	}
	return bytes, nil
}

// SavedReadList is a list read back element by element, only if it holds no more
// than the capacity the machine bounds it by: a machine never holds more, and a
// restored one must not be the first to.
func SavedReadList[T any](
	value any, what string, capacity int, element func(any, string) (T, error),
) ([]T, error) {
	items, ok := value.([]any)
	if !ok {
		return nil, notA(value, what, "an array")
	}
	if len(items) > capacity {
		return nil, refuse("'%s' holds %d elements, past the %d it is bounded by", what, len(items), capacity)
	}
	read := make([]T, len(items))
	for at, item := range items {
		var err error
		if read[at], err = element(item, fmt.Sprintf("%s[%d]", what, at)); err != nil {
			return nil, err
		}
	}
	return read, nil
}

// SavedReadOptional is a value that may be absent read back (docs/adr/0055,
// section 4): absent, as none makes it, where the saved state holds null, and
// otherwise the value present reads, held to its type as any value is, and
// wrapped by some. Only a place the machine declares optional reads through
// this, so a null saved where the machine holds a value of T is refused by the
// reader of T, naming what holds it.
func SavedReadOptional[T any, O any](
	value any,
	what string,
	present func(any, string) (T, error),
	none func() O,
	some func(T) O,
) (O, error) {
	if value == nil {
		return none(), nil
	}
	read, err := present(value, what)
	if err != nil {
		var zero O
		return zero, err
	}
	return some(read), nil
}

// SavedBoundedString is the reader of a text held to capacity UTF-8 bytes, for a
// place that reads through a function value ([SavedReadOptional]).
func SavedBoundedString(capacity int) func(any, string) (string, error) {
	return func(value any, what string) (string, error) {
		return SavedReadBoundedString(value, what, capacity)
	}
}

// SavedBoundedBytes is the reader of a byte string held to capacity bytes, for a
// place that reads through a function value ([SavedReadOptional]).
func SavedBoundedBytes(capacity int) func(any, string) ([]byte, error) {
	return func(value any, what string) ([]byte, error) {
		return SavedReadBytes(value, what, capacity)
	}
}

// SavedField is the field name of a saved record value: what a generated record
// reads each field with.
func SavedField(value any, record, name string) (any, error) {
	members, ok := value.(*JSONObject)
	if !ok {
		return nil, refuse("'%s' is not an object", record)
	}
	field, held := members.Get(name)
	if !held {
		return nil, refuse("'%s' has no field '%s'", record, name)
	}
	return field, nil
}
