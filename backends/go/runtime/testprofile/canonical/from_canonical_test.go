// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// A value read back from the canonical form of each type of the profile: the
// Go half of the Rust module's own tests
// (backends/rust/runtime/src/test/from_canonical.rs), each named after the Rust
// case in Pascal case, with the same spellings and the same refusals. The Rust
// case of a union has no twin, since Go lowers no union, and the one of a
// field given twice has none, since no *sce.JSONObject holds one.

package canonical

import (
	"bytes"
	"math"
	"strings"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
)

// reason is the reason of the refusal err, failing the test where the value
// was read.
func reason(t *testing.T, err error) string {
	t.Helper()
	if err == nil {
		t.Fatal("the value is read where it should be refused")
	}
	return err.Error()
}

// parsed is text read as JSON, failing the test where it is not.
func parsed(t *testing.T, text string) any {
	t.Helper()
	value, err := sce.ParseJSON(text)
	if err != nil {
		t.Fatalf("%s is JSON: %v", text, err)
	}
	return value
}

// An integer is read from the one spelling the writer gives it, at every
// width, and any other spelling, or a value past its type, is refused, naming
// where it stands.
func TestAnIntegerIsReadFromTheTextOfItsDigitsAlone(t *testing.T) {
	if got, err := ReadInteger[uint8](Integer(uint8(0)), "n"); err != nil || got != 0 {
		t.Errorf("0 reads as %d, %v", got, err)
	}
	if got, err := ReadInteger[uint64](Integer(uint64(math.MaxUint64)), "n"); err != nil || got != math.MaxUint64 {
		t.Errorf("the greatest uint64 reads as %d, %v", got, err)
	}
	if got, err := ReadInteger[int64](Integer(int64(math.MinInt64)), "n"); err != nil || got != math.MinInt64 {
		t.Errorf("the least int64 reads as %d, %v", got, err)
	}
	if got, err := ReadInteger[paint](Integer(paint(7)), "n"); err != nil || got != 7 {
		t.Errorf("an enum's carrier reads as %d, %v", got, err)
	}
	for _, spelling := range []string{"007", "+7", "-0", "7.0", "", " 7"} {
		_, err := ReadInteger[uint32](spelling, "job_reply.code")
		refusal := reason(t, err)
		if !strings.HasPrefix(refusal, "job_reply.code: ") || !strings.Contains(refusal, "decimal digits") {
			t.Errorf("%q: %s", spelling, refusal)
		}
	}
	for _, past := range []struct {
		read func() error
		text string
	}{
		{func() error { _, err := ReadInteger[uint8]("256", "n"); return err }, "256"},
		{func() error { _, err := ReadInteger[uint32]("-1", "n"); return err }, "-1"},
		{func() error { _, err := ReadInteger[int8]("-129", "n"); return err }, "-129"},
		{func() error { _, err := ReadInteger[int64]("9223372036854775808", "n"); return err }, "9223372036854775808"},
		{func() error { _, err := ReadInteger[uint64]("18446744073709551616", "n"); return err }, "18446744073709551616"},
	} {
		if got := reason(t, past.read()); got != "n: "+past.text+" is past the range of its type" {
			t.Errorf("%s: %s", past.text, got)
		}
	}
	_, err := ReadInteger[uint32](sce.JSONNumber("7"), "n")
	if got := reason(t, err); !strings.Contains(got, "a number is not an integer") {
		t.Errorf("a JSON number: %s", got)
	}
}

// A text of digits past the range of a 128-bit integer is no integer's text,
// as the Rust reader of it, which reads it as an `i128`, holds; one within it
// is past the range of every type it is read as.
func TestADecimalTextIsWithinTheRangeOfA128BitInteger(t *testing.T) {
	for _, within := range []string{"0", "-1", greatest128, "-" + least128} {
		if !DecimalText(within) {
			t.Errorf("%s spells an integer", within)
		}
	}
	for _, past := range []string{"170141183460469231731687303715884105728", "-170141183460469231731687303715884105729",
		"1701411834604692317316873037158841057270", "-0", "01", "-", "1a"} {
		if DecimalText(past) {
			t.Errorf("%s is no integer's canonical text", past)
		}
	}
	_, err := ReadInteger[uint64]("170141183460469231731687303715884105728", "n")
	if got := reason(t, err); !strings.Contains(got, "is not the text of an integer's decimal digits") {
		t.Errorf("past 128 bits: %s", got)
	}
}

func TestABoolIsATruthValue(t *testing.T) {
	if got, err := ReadBoolean(Boolean(true), "b"); err != nil || !got {
		t.Errorf("true reads as %t, %v", got, err)
	}
	_, err := ReadBoolean("true", "b")
	if got := reason(t, err); got != "b: a text is not a truth value" {
		t.Errorf("a text: %s", got)
	}
}

// A text is read within its bound in bytes.
func TestATextIsReadWithinItsBound(t *testing.T) {
	if got, err := ReadText(Text("abc"), 3, "t"); err != nil || got != "abc" {
		t.Errorf("abc reads as %q, %v", got, err)
	}
	_, err := ReadText(Text("abcd"), 3, "t")
	if got := reason(t, err); got != "t: a text of 4 bytes is past the 3 its type holds" {
		t.Errorf("past the bound: %s", got)
	}
	_, err = ReadText(nil, 3, "t")
	if got := reason(t, err); got != "t: null is not a text" {
		t.Errorf("null: %s", got)
	}
}

// A byte string is read from upper-case hex alone, the writer's spelling, and
// back to the same text.
func TestAByteStringIsReadFromUpperCaseHex(t *testing.T) {
	written := Bytes([]byte{0x00, 0xab, 0x0f, 0xff})
	read, err := ReadBytes(written, 4, "b")
	if err != nil || !bytes.Equal(read, []byte{0x00, 0xab, 0x0f, 0xff}) {
		t.Errorf("%s reads as %v, %v", written, read, err)
	}
	if Bytes(read) != written {
		t.Errorf("%s is written back as %s", written, Bytes(read))
	}
	for _, spelling := range []string{"00ab", "0", "0G"} {
		_, err := ReadBytes(spelling, 4, "b")
		if got := reason(t, err); !strings.Contains(got, "upper-case hex") {
			t.Errorf("%q: %s", spelling, got)
		}
	}
	_, err = ReadBytes(written, 3, "b")
	if got := reason(t, err); got != "b: a byte string of 4 octets is past the 3 its type holds" {
		t.Errorf("past the bound: %s", got)
	}
}

// A closed enum is read by its declared names; an open one by its names and by
// the numbers no variant declares, a declared number being the second spelling
// of its variant.
func TestAnEnumIsReadByItsDeclaredName(t *testing.T) {
	declared := []Variant[paint]{{Name: "red", Value: 0}, {Name: "green", Value: 1}}
	if got, err := ReadEnumerated("green", declared, "c"); err != nil || got != 1 {
		t.Errorf("green reads as %d, %v", got, err)
	}
	_, err := ReadEnumerated("blue", declared, "c")
	if got := reason(t, err); got != "c: 'blue' names no variant of its enum" {
		t.Errorf("blue: %s", got)
	}
	open := func(raw uint8) paint { return paint(raw) }
	if got, err := ReadOpenEnumerated("red", declared, open, "c"); err != nil || got != 0 {
		t.Errorf("red reads as %d, %v", got, err)
	}
	if got, err := ReadOpenEnumerated("7", declared, open, "c"); err != nil || got != 7 {
		t.Errorf("7 reads as %d, %v", got, err)
	}
	_, err = ReadOpenEnumerated("1", declared, open, "c")
	if got := reason(t, err); got != "c: 1 is the number of a declared variant, which is written by its name" {
		t.Errorf("a declared number: %s", got)
	}
	_, err = ReadOpenEnumerated("300", declared, open, "c")
	if got := reason(t, err); got != "c: 300 is past the range of its enum" {
		t.Errorf("past the carrier: %s", got)
	}
	_, err = ReadOpenEnumerated("hexagon", declared, open, "c")
	if got := reason(t, err); got != "c: 'hexagon' names no variant of its enum, and is no number" {
		t.Errorf("no name and no number: %s", got)
	}
}

// A record holds only the fields it declares; a field whose type is not
// optional is present, and one that is absent is refused where it is read.
func TestARecordHoldsItsDeclaredFields(t *testing.T) {
	object, err := ReadRecord(parsed(t, `{"code":"0","job":"7"}`), []string{"job", "code"}, "job_reply")
	if err != nil {
		t.Fatalf("a record: %v", err)
	}
	if got, err := ReadInteger[uint32](Member(object, "job"), "job_reply.job"); err != nil || got != 7 {
		t.Errorf("job reads as %d, %v", got, err)
	}
	if _, present := object.Get("note"); present {
		t.Error("a field the object does not name is absent")
	}
	_, err = ReadRecord(parsed(t, `{"job":"7","jobs":"1"}`), []string{"job", "code"}, "job_reply")
	if got := reason(t, err); got != "job_reply: 'jobs' is no field of its record" {
		t.Errorf("a stray field: %s", got)
	}
	short, err := ReadRecord(parsed(t, `{"job":"7"}`), []string{"job", "code"}, "job_reply")
	if err != nil {
		t.Fatalf("a record: %v", err)
	}
	_, err = ReadInteger[uint32](Member(short, "code"), "job_reply.code")
	if got := reason(t, err); got != "job_reply.code: the field is absent, and its type is not optional" {
		t.Errorf("an absent field: %s", got)
	}
	_, err = ReadRecord([]any{}, []string{"job"}, "job_reply")
	if got := reason(t, err); got != "job_reply: an array is not a record, which is an object of its fields" {
		t.Errorf("an array: %s", got)
	}
}

// A list is read within its bounds, an absent element as null.
func TestAListIsReadWithinItsBounds(t *testing.T) {
	items, err := ReadElements(parsed(t, `["1",null]`), 1, 2, "l")
	if err != nil {
		t.Fatalf("within its bounds: %v", err)
	}
	if got, err := ReadInteger[uint8](items[0], "l[]"); err != nil || got != 1 {
		t.Errorf("the first element reads as %d, %v", got, err)
	}
	if items[1] != nil {
		t.Errorf("an absent element is null: %v", items[1])
	}
	_, err = ReadElements(items, 3, 4, "l")
	if got := reason(t, err); got != "l: a list of 2 elements is outside its bounds, 3 to 4" {
		t.Errorf("outside its bounds: %s", got)
	}
	_, err = ReadElements(nil, 0, 2, "l")
	if got := reason(t, err); got != "l: null is not a list, which is an array" {
		t.Errorf("null: %s", got)
	}
}
