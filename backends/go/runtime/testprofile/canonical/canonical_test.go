// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The canonical form of each type of the profile, written in the JSON
// Canonicalization Scheme: the Go half of the Rust module's own tests
// (backends/rust/runtime/src/test/canonical.rs), each named after the Rust case
// in Pascal case, with the cases only a Go value can call for.

package canonical

import (
	"math"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
)

// written is value in the scheme, failing the test where it is refused.
func written(t *testing.T, value any) string {
	t.Helper()
	text, err := sce.WriteJCS(value)
	if err != nil {
		t.Fatalf("a canonical value is written: %v", err)
	}
	return text
}

// expect holds that value is written as want.
func expect(t *testing.T, value any, want string) {
	t.Helper()
	if got := written(t, value); got != want {
		t.Errorf("wrote %s, want %s", got, want)
	}
}

// name is a DeclaredName: the name and whether there is one.
type name struct {
	declared string
	named    bool
}

func (n name) Get() (string, bool) {
	return n.declared, n.named
}

// paint is an enum's Go type, an integer type of its own.
type paint uint8

// An integer is the text of its digits at every width, the ends of the 64-bit
// ones among them, which a double does not hold exactly, and an enum's carrier
// by its underlying type.
func TestAnIntegerIsTheTextOfItsDecimalDigits(t *testing.T) {
	expect(t, Integer(uint8(0)), `"0"`)
	expect(t, Integer(int8(-128)), `"-128"`)
	expect(t, Integer(uint64(math.MaxUint64)), `"18446744073709551615"`)
	expect(t, Integer(int64(math.MinInt64)), `"-9223372036854775808"`)
	expect(t, Integer(paint(7)), `"7"`)
}

func TestABoolIsATruthValueAndATextAText(t *testing.T) {
	expect(t, Boolean(true), "true")
	expect(t, Boolean(false), "false")
	expect(t, Text("a\"b"), `"a\"b"`)
}

// Two hex digits an octet, upper case, the leading zero kept.
func TestAByteStringIsItsOctetsInUpperCaseHex(t *testing.T) {
	expect(t, Bytes([]byte{0x00, 0xab, 0x0f, 0xff}), `"00AB0FFF"`)
	expect(t, Bytes(nil), `""`)
}

// A closed enum by its declared name; an open one by its name, or by its
// number where no variant declares it.
func TestAnEnumIsItsDeclaredName(t *testing.T) {
	expect(t, Enumerated(name{declared: "red", named: true}), `"red"`)
	expect(t, OpenEnumerated(name{declared: "red", named: true}, uint8(0)), `"red"`)
	expect(t, OpenEnumerated(name{}, paint(7)), `"7"`)
}

// A value of a closed enum's Go type that no variant declares is not a value
// of the enum, and has no form.
func TestAClosedEnumsUndeclaredNumberHasNoForm(t *testing.T) {
	defer func() {
		if recover() == nil {
			t.Fatal("an undeclared number of a closed enum was written")
		}
	}()
	Enumerated(name{})
}

// An absent field is omitted, and the fields are sorted when written, whatever
// order they were given in.
func TestARecordOmitsAnAbsentField(t *testing.T) {
	value := Record(
		Field("quantity", Integer(uint16(2))),
		Field("note", nil),
		Field("color", Enumerated(name{declared: "red", named: true})),
	)
	expect(t, value, `{"color":"red","quantity":"2"}`)
	expect(t, Record(Field("note", nil)), `{}`)
}

// A field that may be absent is written where it is present and absent where
// it is not, and what an absent field holds is never read.
func TestAnOptionalFieldIsReadOnlyWhereItIsPresent(t *testing.T) {
	read := 0
	note := func() any {
		read++
		return Text("late")
	}
	expect(t, Record(Field("note", Optional(true, note))), `{"note":"late"}`)
	expect(t, Record(Field("note", Optional(false, note))), `{}`)
	if read != 1 {
		t.Errorf("the value was read %d times, once for the present field alone", read)
	}
}

func TestAListIsItsElementsInOrder(t *testing.T) {
	codes := []uint8{3, 1, 2}
	expect(t, List(len(codes), func(index int) any { return Integer(codes[index]) }), `["3","1","2"]`)
	expect(t, List(0, func(int) any { return Text("never") }), `[]`)
}

// An absent element of a list is null in its place, and a present one its
// value: the array keeps every position, and what an absent element holds is
// never read.
func TestAnAbsentElementOfAListIsNull(t *testing.T) {
	present := []bool{true, false}
	held := []uint8{1, 0}
	read := 0
	element := func(index int) any {
		return OptionalElement(present[index], func() any {
			read++
			return Integer(held[index])
		})
	}
	expect(t, List(len(present), element), `["1",null]`)
	if read != 1 {
		t.Errorf("the elements were read %d times, once for the present one alone", read)
	}
}

// A union is one member named by its case's variant: the case's record, or the
// empty object for a case that holds none.
func TestAUnionIsOneMemberNamedByItsVariant(t *testing.T) {
	card := UnionCase("card", Record(Field("amount", Integer(uint32(1)))))
	expect(t, card, `{"card":{"amount":"1"}}`)
	expect(t, UnionCase("cash", nil), `{"cash":{}}`)
}
