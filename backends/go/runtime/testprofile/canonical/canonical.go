// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Package canonical writes the canonical form of a message's value in the test
// log (docs/SCE_TEST_PROFILE.md §12.5; docs/adr/0091, C3 and answer 8): one
// JSON value per value of the profile's types, built from texts, truth values,
// arrays and objects alone, so that the JSON Canonicalization Scheme
// (sce.WriteJCS) writes two equal values as the same bytes on every runtime.
//
//   - an integer of any width is the text of its decimal digits, since an
//     `int64` or a `uint64` is past what a JSON reader that holds numbers as
//     doubles reads exactly;
//   - a `bool` is a truth value;
//   - a value of an enum is the name its document declares for its variant;
//   - a text is a text;
//   - a byte string is the text of its octets in upper-case hex, two digits
//     each;
//   - a record is an object of its fields by their declared ids, a field that
//     is absent omitted, as TTCN-3's `omit` leaves it out;
//   - a list is an array of its elements in order.
//
// A value is held as the JSON value model of the runtime's JSON writers: a
// string, a bool, a []any or a *sce.JSONObject.
//
// The generated unit of a test template over a record spells its `Canonical`
// with these, by the types its document declares
// (`forge/go/test_template.go.jinja2`): a Go type alone does not say which
// form a value takes, since a byte string and a list of `uint8` are one Go
// type. Go lowers no union, so no form of one is written here.
//
// Cross-language siblings: `sce_rust_runtime::test::canonical` (Rust),
// `SCE::Test::Canonical` (C++), `com.sce.runtime.test.Canonical` (Kotlin).
package canonical

import (
	"strconv"

	sce "github.com/newmassrael/sce-go-runtime"
)

// Integral is every integer type a value of the profile is held in, the
// carrier of an enum among them, by its underlying type.
type Integral interface {
	~int | ~int8 | ~int16 | ~int32 | ~int64 | ~uint | ~uint8 | ~uint16 | ~uint32 | ~uint64
}

// Integer is an integer of any width the profile has: the text of its decimal
// digits.
func Integer[N Integral](value N) string {
	if value < 0 {
		return strconv.FormatInt(int64(value), 10)
	}
	return strconv.FormatUint(uint64(value), 10)
}

// Boolean is a `bool`.
func Boolean(value bool) bool {
	return value
}

// Text is a text.
func Text(value string) string {
	return value
}

// upperHex are the sixteen digits of an octet, upper case.
const upperHex = "0123456789ABCDEF"

// Bytes is a byte string: its octets in upper-case hex, two digits each, as
// TTCN-3 writes an `octetstring` (`'00FF'O`) without its quotes and its `O`.
func Bytes(value []byte) string {
	out := make([]byte, 0, 2*len(value))
	for _, octet := range value {
		out = append(out, upperHex[octet>>4], upperHex[octet&0x0F])
	}
	return string(out)
}

// Named is what a generated enum's DeclaredName answers
// (`forge/go/enum.go.jinja2`), read by its Get: the name the document declares
// for the value and true, or false for a number no variant declares.
type Named interface {
	Get() (string, bool)
}

// Enumerated is a value of a closed enum: the name its document declares for
// the variant. A Go value of a closed enum's type can hold a number no variant
// declares, which is not a value of the enum (its FromUnderlying refuses it),
// so it has no canonical form and panics, as the C++ writer throws.
func Enumerated(declared Named) string {
	name, named := declared.Get()
	if !named {
		panic("a value of a closed enum that no variant declares has no canonical form")
	}
	return name
}

// OpenEnumerated is a value of an enum of an open set
// (`sce:strict-variants="false"`): the name its document declares for it, or,
// for a number no variant declares, the text of that number's decimal digits.
// No variant's name is such a text, since a name is an identifier. The TTCN-3
// export refuses an enum of an open set (docs/SCE_TEST_PROFILE.md §9.5), so
// only a native log holds this form.
func OpenEnumerated[N Integral](declared Named, number N) string {
	if name, named := declared.Get(); named {
		return name
	}
	return Integer(number)
}

// RecordField is one field of a record in the canonical form: its declared id
// and its value, or nil where the field is absent.
type RecordField struct {
	id    string
	value any
}

// Field is the field id of a record holding value, or absent where value is nil
// (Optional).
func Field(id string, value any) RecordField {
	return RecordField{id: id, value: value}
}

// Record is a record: each field by its declared id, in any order, since the
// scheme sorts an object's members; a field whose value is nil is absent and
// omitted. No value of the canonical form is nil, so nil means absence alone.
func Record(fields ...RecordField) *sce.JSONObject {
	members := sce.NewJSONObject()
	for _, field := range fields {
		if field.value != nil {
			members.Set(field.id, field.value)
		}
	}
	return members
}

// Optional is a field that may be absent: the value write gives where present
// holds, and nil, which Record omits, where it does not. write is called only
// for a present field, since what an absent one holds is no value of its type
// (a closed enum's zero, which no variant need declare, would panic).
func Optional(present bool, write func() any) any {
	if !present {
		return nil
	}
	return write()
}

// List is a list of count elements in order, each in the form element gives the
// one at its index.
func List(count int, element func(index int) any) []any {
	items := make([]any, 0, count)
	for index := 0; index < count; index++ {
		items = append(items, element(index))
	}
	return items
}
