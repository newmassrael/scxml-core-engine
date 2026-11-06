// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A payload value of any bounded type is read at its path, or refused there
// before anything of it is kept (docs/adr/0086, sections 4 and 14.3): every
// element kind, at list depths 1 to 3, each bound 2, with an empty value, a
// maximal one, one past the bound at the innermost and the outermost list, a
// bad element, `null` where the element is not optional, an unknown key and an
// older revision. The oracle is the construction: each case is built with the
// value or the path it must come to. A maximal value is written at exactly the
// size the calculus gives it, and reads back as itself.

package sce

import (
	"fmt"
	"math"
	"reflect"
	"strings"
	"testing"
)

// The bound of every list of the cross product, and of a text or byte string.
const (
	crossBound = 2
	textBound  = 3
)

type crossKind int

const (
	kU8 crossKind = iota
	kU16
	kU32
	kU64
	kI8
	kI16
	kI32
	kI64
	kF32
	kF64
	kBool
	kEnum
	kText
	kBytes
	kOpt
	kRec
	kRecList
)

var crossKindNames = []string{
	"uint8", "uint16", "uint32", "uint64", "int8", "int16", "int32", "int64",
	"float32", "float64", "bool", "enum", "string[3]", "bytes[3]", "optional<uint8>",
	"record", "record of a list",
}

// crossType is a kind, or a list of a crossType when elem is set. kRec is the
// record `{a: uint8, b: string[3]}` whose schema's later revision added `c:
// uint8` with the default 9; kRecList is `{tags: list<string[3]>[2]}`; kEnum
// declares `low`, `medium`, `high`.
type crossType struct {
	kind crossKind
	elem *crossType
}

func nested(kind crossKind, depth int) crossType {
	t := crossType{kind: kind}
	for i := 0; i < depth; i++ {
		inner := t
		t = crossType{elem: &inner}
	}
	return t
}

// absentValue is an absent optional of the model.
type absentValue struct{}

// member is one field of a record of the model, in id order.
type member struct {
	id    string
	value any
}

var enumNames = []string{"low", "medium", "high"}

// readCross is the reader of t, one call per constructor, as the generator
// writes it.
func readCross(t crossType, v PayloadValue) (any, error) {
	if t.elem != nil {
		return ReadList(v, crossBound, func(e PayloadValue) (any, error) { return readCross(*t.elem, e) })
	}
	switch t.kind {
	case kU8:
		n, err := ValueUnsigned[uint8](v)
		return uint64(n), err
	case kU16:
		n, err := ValueUnsigned[uint16](v)
		return uint64(n), err
	case kU32:
		n, err := ValueUnsigned[uint32](v)
		return uint64(n), err
	case kU64:
		return ValueUnsigned64(v)
	case kI8:
		n, err := ValueSigned[int8](v)
		return int64(n), err
	case kI16:
		n, err := ValueSigned[int16](v)
		return int64(n), err
	case kI32:
		n, err := ValueSigned[int32](v)
		return int64(n), err
	case kI64:
		return ValueSigned64(v)
	case kF32:
		x, err := ValueFloat[float32](v)
		return float64(x), err
	case kF64:
		return ValueFloat[float64](v)
	case kBool:
		return v.Bool()
	case kEnum:
		name, err := v.Name()
		if err != nil {
			return nil, err
		}
		for _, declared := range enumNames {
			if declared == name {
				return name, nil
			}
		}
		return nil, v.UnknownVariant(name, "Level")
	case kText:
		return v.Text(textBound)
	case kBytes:
		return v.Bytes(textBound)
	case kOpt:
		if v.IsNull() {
			return absentValue{}, nil
		}
		n, err := ValueUnsigned[uint8](v)
		return uint64(n), err
	case kRec:
		r, err := v.Record(`{"c":9}`)
		if err != nil {
			return nil, err
		}
		fields := []member{}
		for _, id := range []string{"a", "b", "c"} {
			f, err := r.Field(id)
			if err != nil {
				return nil, err
			}
			var held any
			if id == "b" {
				held, err = f.Text(textBound)
			} else {
				var n uint8
				n, err = ValueUnsigned[uint8](f)
				held = uint64(n)
			}
			if err != nil {
				return nil, err
			}
			fields = append(fields, member{id, held})
		}
		return fields, nil
	case kRecList:
		r, err := v.Record("")
		if err != nil {
			return nil, err
		}
		f, err := r.Field("tags")
		if err != nil {
			return nil, err
		}
		tags, err := readCross(nested(kText, 1), f)
		if err != nil {
			return nil, err
		}
		return []member{{"tags", tags}}, nil
	}
	panic("no reader for the kind")
}

// controls is a text of n control characters, each of which JSON spells at its
// longest.
func controls(n int) string { return strings.Repeat("\u0001", n) }

// maximal is the maximal value of t: every list full, every text at its bound
// of control characters, every number at its longest spelling.
func maximal(t crossType) any {
	if t.elem != nil {
		items := make([]any, crossBound)
		for i := range items {
			items[i] = maximal(*t.elem)
		}
		return items
	}
	switch t.kind {
	case kU8:
		return uint64(math.MaxUint8)
	case kU16:
		return uint64(math.MaxUint16)
	case kU32:
		return uint64(math.MaxUint32)
	case kU64:
		return uint64(math.MaxUint64)
	case kI8:
		return int64(math.MinInt8)
	case kI16:
		return int64(math.MinInt16)
	case kI32:
		return int64(math.MinInt32)
	case kI64:
		return int64(math.MinInt64)
	// Seventeen digits after `0.00000`: the longest spelling of a number,
	// which only the decimal band from 1e-6 to 1e-5 reaches.
	case kF32:
		return float64(math.Float32frombits(0xb5a5b36d))
	case kF64:
		return -math.Float64frombits(0x3eb4b66dc01ec6fb)
	case kBool:
		return false
	case kEnum:
		return "medium"
	case kText:
		return controls(textBound)
	case kBytes:
		return []byte{1, 1, 1}
	case kOpt:
		// `null` (4 bytes) is longer than any uint8's digits.
		return absentValue{}
	case kRec:
		return []member{{"a", uint64(math.MaxUint8)}, {"b", controls(textBound)}, {"c", uint64(math.MaxUint8)}}
	case kRecList:
		return []member{{"tags", []any{controls(textBound), controls(textBound)}}}
	}
	panic("no maximal value for the kind")
}

// writeCross is held as JSON, written by the runtime's writers, one call per
// constructor; a record's members are handed in reverse to show that the writer
// orders them.
func writeCross(t crossType, held any) RawJSON {
	if t.elem != nil {
		return ListJSON(held.([]any), func(e any) RawJSON { return writeCross(*t.elem, e) })
	}
	switch v := held.(type) {
	case uint64:
		if t.kind == kU64 {
			return ValueJSON(Uint64PayloadText(v))
		}
		return ValueJSON(v)
	case int64:
		if t.kind == kI64 {
			return ValueJSON(Int64PayloadText(v))
		}
		return ValueJSON(v)
	case []byte:
		return ValueJSON(BytesAsPayloadText(v))
	case absentValue:
		return ValueJSON(nil)
	case []member:
		pairs := make([]PayloadPair, 0, len(v))
		for i := len(v) - 1; i >= 0; i-- {
			field := crossType{kind: kU8}
			switch v[i].id {
			case "b":
				field = crossType{kind: kText}
			case "tags":
				field = nested(kText, 1)
			}
			pairs = append(pairs, Pair(v[i].id, writeCross(field, v[i].value)))
		}
		return RecordJSON(pairs...)
	default:
		return ValueJSON(v)
	}
}

// largest is the largest JSON text of t (docs/adr/0086, section 2.1).
func largest(t crossType) int {
	list := func(elem int) int { return 2 + crossBound*elem + (crossBound - 1) }
	record := func(ids []string, sizes []int) int {
		total := 2 + len(ids) - 1
		for i, id := range ids {
			total += len(id) + 2 + 1 + sizes[i]
		}
		return total
	}
	if t.elem != nil {
		return list(largest(*t.elem))
	}
	text := 2 + 6*textBound
	switch t.kind {
	case kU8:
		return 3
	case kU16:
		return 5
	case kU32:
		return 10
	case kU64, kI64:
		return 22
	case kI8:
		return 4
	case kI16:
		return 6
	case kI32:
		return 11
	case kF32, kF64:
		return 25
	case kBool:
		return 5
	case kEnum:
		return 2 + len("medium")
	case kText, kBytes:
		return text
	case kOpt:
		return 4
	case kRec:
		return record([]string{"a", "b", "c"}, []int{3, text, 3})
	case kRecList:
		return record([]string{"tags"}, []int{list(text)})
	}
	panic("no size for the kind")
}

func crossData(list RawJSON) string {
	return `{"v":` + string(list) + `,"zz":{"unknown":[1,null]}}`
}

// innermost is the path of the innermost list along the last index at each
// depth.
func innermost(depth int) string {
	path := "v"
	for i := 1; i < depth; i++ {
		path += fmt.Sprintf("/%d", crossBound-1)
	}
	return path
}

// withLastLeaf is held with the element at the end of the last index at every
// depth replaced by leaf.
func withLastLeaf(t crossType, held any, leaf string) RawJSON {
	if t.elem == nil {
		return RawJSON(leaf)
	}
	items := held.([]any)
	parts := make([]string, len(items))
	for i, item := range items {
		parts[i] = string(writeCross(*t.elem, item))
	}
	parts[len(parts)-1] = string(withLastLeaf(*t.elem, items[len(items)-1], leaf))
	return RawJSON("[" + strings.Join(parts, ",") + "]")
}

// onePast is the maximal value with one more element in the list at level (1
// the outermost) along the last index.
func onePast(t crossType, held any, level int) RawJSON {
	items := held.([]any)
	parts := make([]string, len(items))
	for i, item := range items {
		parts[i] = string(writeCross(*t.elem, item))
	}
	if level == 1 {
		parts = append(parts, string(writeCross(*t.elem, items[0])))
	} else {
		parts[len(parts)-1] = string(onePast(*t.elem, items[len(items)-1], level-1))
	}
	return RawJSON("[" + strings.Join(parts, ",") + "]")
}

// setLastLeaf is held with its last leaf replaced by leaf.
func setLastLeaf(held any, leaf any) any {
	items, ok := held.([]any)
	if !ok {
		return leaf
	}
	out := append([]any(nil), items...)
	out[len(out)-1] = setLastLeaf(out[len(out)-1], leaf)
	return out
}

// badElement is a bad element of a kind, the path below the element it is
// refused at, and the sentence it is refused with.
func badElement(kind crossKind) (string, string, string) {
	switch kind {
	case kU8, kU16, kU32, kOpt:
		return "256000000000", "", "does not fit the width"
	case kU64:
		return `"-1"`, "", "is not a whole number written as digits"
	case kI8, kI16, kI32:
		return "1.5", "", "is not a whole number"
	case kI64:
		return "9007199254740992", "", "past 2^53 - 1"
	case kF32:
		return "1e39", "", "does not fit the width"
	case kF64:
		return "true", "", "is not a number"
	case kBool:
		return "0", "", "is not a truth value"
	case kEnum:
		return `"lowest"`, "", "is not a variant of Level"
	case kText:
		return `"abcd"`, "", "is 4 bytes, past the 3 its type declares"
	case kBytes:
		return `"Ā"`, "", "carries a character above U+00FF"
	case kRec:
		return `{"a":"x","b":""}`, "/a", "is not a number"
	case kRecList:
		return `{"tags":["","","x"]}`, "/tags", "holds 3 elements, past the 2"
	}
	panic("no bad element for the kind")
}

func liftCross(t crossType, text string) (any, error) {
	fields, err := LiftPayload(text)
	if err != nil {
		return nil, err
	}
	v, err := fields.Field("v")
	if err != nil {
		return nil, err
	}
	return readCross(t, v)
}

// refusedAt is a case that must be refused at path, saying sentence.
type refusedAt struct{ path, sentence string }

func TestEveryElementKindAtEveryDepthIsReadOrRefusedAtItsPath(t *testing.T) {
	cases := 0
	expect := func(label string, got any, err error, want any) {
		t.Helper()
		cases++
		if refused, ok := want.(refusedAt); ok {
			if err == nil || !strings.Contains(err.Error(), "'"+refused.path+"'") ||
				!strings.Contains(err.Error(), refused.sentence) {
				t.Errorf("%s: read %v (%v), the oracle says refused at '%s': %s", label, got, err,
					refused.path, refused.sentence)
			}
			return
		}
		if err != nil || !reflect.DeepEqual(got, want) {
			t.Errorf("%s: read %#v (%v), the oracle says %#v", label, got, err, want)
		}
	}
	pastBound := "holds 3 elements, past the 2 its type declares"
	for kind := kU8; kind <= kRecList; kind++ {
		for depth := 1; depth <= 3; depth++ {
			ty := nested(kind, depth)
			label := func(c string) string { return fmt.Sprintf("%s at depth %d, %s", crossKindNames[kind], depth, c) }
			max := maximal(ty)
			leaf := fmt.Sprintf("%s/%d", innermost(depth), crossBound-1)

			got, err := liftCross(ty, crossData("[]"))
			expect(label("empty"), got, err, []any{})
			got, err = liftCross(ty, crossData(writeCross(ty, max)))
			expect(label("maximal"), got, err, max)
			got, err = liftCross(ty, crossData(onePast(ty, max, depth)))
			expect(label("one past the innermost bound"), got, err, refusedAt{innermost(depth), pastBound})
			got, err = liftCross(ty, crossData(onePast(ty, max, 1)))
			expect(label("one past the outermost bound"), got, err, refusedAt{"v", pastBound})
			if kind == kOpt {
				// The maximal list of optionals is every element absent; a
				// present one among them is read as present.
				got, err = liftCross(ty, crossData(withLastLeaf(ty, max, "7")))
				expect(label("a present element among absent ones"), got, err, setLastLeaf(max, uint64(7)))
			} else {
				got, err = liftCross(ty, crossData(withLastLeaf(ty, max, "null")))
				expect(label("null where the element is not optional"), got, err,
					refusedAt{leaf, "is null, and its type is not optional"})
			}
			text, below, sentence := badElement(kind)
			got, err = liftCross(ty, crossData(withLastLeaf(ty, max, text)))
			expect(label("a bad element"), got, err, refusedAt{leaf + below, sentence})
			// A key no schema declares, inside a record element and beside the
			// field, is stepped over.
			unknown := writeCross(ty, max)
			if kind == kRec {
				unknown = withLastLeaf(ty, max, `{"zz":[{"q":1}],"c":255,"b":"\u0001\u0001\u0001","a":255}`)
			}
			got, err = liftCross(ty, crossData(unknown))
			expect(label("an unknown key"), got, err, max)
			// An older revision: a record element without the field its schema's
			// later revision added reads that field's default, and a payload
			// without a list a later revision added reads it empty.
			if kind == kRec {
				older := setLastLeaf(max, []member{{"a", uint64(1)}, {"b", ""}, {"c", uint64(9)}})
				got, err = liftCross(ty, crossData(withLastLeaf(ty, max, `{"a":1,"b":""}`)))
				expect(label("an element of an older revision"), got, err, older)
			} else {
				fields, liftErr := LiftPayloadWithDefaults(`{"zz":1}`, `{"v":[]}`)
				got, err = nil, liftErr
				if liftErr == nil {
					var v PayloadValue
					if v, err = fields.Field("v"); err == nil {
						got, err = readCross(ty, v)
					}
				}
				expect(label("a delivery of the revision before the list"), got, err, []any{})
			}
		}
	}
	if want := 17 * 3 * 8; cases != want {
		t.Errorf("the cross product holds %d cases, want %d", cases, want)
	}
}

func TestAMaximalValueIsWrittenAtExactlyItsLargestSize(t *testing.T) {
	for kind := kU8; kind <= kRecList; kind++ {
		for depth := 0; depth <= 3; depth++ {
			ty := nested(kind, depth)
			max := maximal(ty)
			text := writeCross(ty, max)
			size := largest(ty)
			if len(text) != size {
				t.Errorf("%s at depth %d: %d bytes, the calculus says %d: %s",
					crossKindNames[kind], depth, len(text), size, text)
			}
			back, err := liftCross(ty, crossData(text))
			if err != nil || !reflect.DeepEqual(back, max) {
				t.Errorf("%s at depth %d: written and read back as %#v (%v)", crossKindNames[kind], depth, back, err)
			}
		}
	}
}

func TestATopLevelTextAndByteStringAreHeldToTheirBoundWhenRead(t *testing.T) {
	fields, err := LiftPayload(`{"note": "abcde", "blob": "abcde", "t": "éé", "n": null}`)
	if err != nil {
		t.Fatal(err)
	}
	note, _ := fields.Field("note")
	if _, err := note.Text(4); err == nil || err.Error() != "'note' is 5 bytes, past the 4 its type declares" {
		t.Errorf("a text past its bound reads %v", err)
	}
	blob, _ := fields.Field("blob")
	if _, err := blob.Bytes(4); err == nil || err.Error() != "'blob' is 5 bytes, past the 4 its type declares" {
		t.Errorf("a byte string past its bound reads %v", err)
	}
	// A text's bound counts UTF-8 bytes: two characters, four bytes.
	wide, _ := fields.Field("t")
	if _, err := wide.Text(3); err == nil {
		t.Errorf("four bytes were read into a bound of three")
	}
	if got, err := wide.Text(4); err != nil || got != "éé" {
		t.Errorf("four bytes in a bound of four read %q, %v", got, err)
	}
	null, _ := fields.Field("n")
	if _, err := ValueSigned64(null); err == nil || err.Error() != "'n' is null, and its type is not optional" {
		t.Errorf("null where the type is not optional reads %v", err)
	}
	if _, err := fields.Field("gone"); err == nil || err.Error() != "the event's data has no 'gone'" {
		t.Errorf("a missing field reads %v", err)
	}
}

func TestAListPastItsBoundIsRefusedBeforeAnElementIsRead(t *testing.T) {
	fields, err := LiftPayload(`{"v": [1, 2, "x"]}`)
	if err != nil {
		t.Fatal(err)
	}
	v, _ := fields.Field("v")
	asked := 0
	_, err = ReadList(v, 2, func(e PayloadValue) (uint8, error) {
		asked++
		return ValueUnsigned[uint8](e)
	})
	if err == nil || err.Error() != "'v' holds 3 elements, past the 2 its type declares" {
		t.Errorf("three elements in a bound of two read %v", err)
	}
	if asked != 0 {
		t.Errorf("%d elements were read before the bound was held", asked)
	}
	notList, _ := fields.Field("v")
	notList.value = "x"
	if _, err := ReadList(notList, 2, ValueUnsigned[uint8]); err == nil || err.Error() != "'v' is not a list" {
		t.Errorf("a text where a list stands reads %v", err)
	}
}

// A list short of its lower count is refused as tooFew, naming the lower count,
// before any element is read; one that holds it is read (docs/adr/0086, section
// 15.5).
func TestAListShortOfItsLowerCountIsRefusedBeforeAnElementIsRead(t *testing.T) {
	fields, err := LiftPayload(`{"v": ["x"], "w": [1, 2], "e": []}`)
	if err != nil {
		t.Fatal(err)
	}
	v, _ := fields.Field("v")
	asked := 0
	_, err = ReadListWithin(v, 2, 3, func(e PayloadValue) (uint8, error) {
		asked++
		return ValueUnsigned[uint8](e)
	})
	refusal, ok := err.(*PayloadRefusal)
	if !ok || refusal.Error() != "'v' holds 1 elements, fewer than the 2 its type declares" {
		t.Fatalf("one element short of two reads %v", err)
	}
	if bound, has := refusal.Bound(); refusal.Code() != RefusalTooFew || refusal.Pointer() != "/v" || !has || bound != 2 {
		t.Errorf("the refusal is %v at %q bound %d (%v)", refusal.Code(), refusal.Pointer(), bound, has)
	}
	if asked != 0 {
		t.Errorf("%d elements were read before the count was held", asked)
	}
	w, _ := fields.Field("w")
	if held, err := ReadListWithin(w, 2, 3, ValueUnsigned[uint8]); err != nil || len(held) != 2 {
		t.Errorf("two elements of at least two read %v, %v", held, err)
	}
	e, _ := fields.Field("e")
	if _, err := ReadListWithin(e, 1, 3, ValueUnsigned[uint8]); err == nil ||
		err.(*PayloadRefusal).Code() != RefusalTooFew {
		t.Errorf("the empty list where one is needed reads %v", err)
	}
	if _, err := ReadList(e, 3, ValueUnsigned[uint8]); err != nil {
		t.Errorf("the empty list where none is needed reads %v", err)
	}
}

// The writer's check refuses a list short of the place's lower count in the
// reader's words, the path in front (docs/adr/0086, section 15.6).
func TestAListWrittenShortOfItsLowerCountIsRefused(t *testing.T) {
	none := func(string, uint8) error { return nil }
	if err := CheckListWithin("items", []uint8{1, 2}, 2, 3, none); err != nil {
		t.Errorf("a list that holds its lower count was refused: %v", err)
	}
	if err := CheckListWithin("items/1/tags", []uint8{}, 1, 3, none); err == nil ||
		err.Error() != "'items/1/tags' holds 0 elements, fewer than the 1 its type declares" {
		t.Errorf("a list short of its lower count reads %v", err)
	}
	if err := CheckListWithin("items", []uint8{1, 2, 3, 4}, 2, 3, none); err == nil ||
		err.Error() != "'items' holds 4 elements, past the 3 its type declares" {
		t.Errorf("a list past its bound reads %v", err)
	}
}

func TestARecordValueNamesItsMissingFieldAndReadsItsLaterFieldsFromDefaults(t *testing.T) {
	fields, err := LiftPayload(`{"r": {"a": 1, "zz": [null]}, "s": 5}`)
	if err != nil {
		t.Fatal(err)
	}
	r, _ := fields.Field("r")
	rec, err := r.Record("")
	if err != nil {
		t.Fatal(err)
	}
	if _, err := rec.Field("b"); err == nil || err.Error() != "the event's data has no 'r/b'" {
		t.Errorf("a missing field of a record reads %v", err)
	}
	revised, err := r.Record(`{"b": 9}`)
	if err != nil {
		t.Fatal(err)
	}
	b, err := revised.Field("b")
	if n, readErr := ValueUnsigned[uint8](b); err != nil || readErr != nil || n != 9 {
		t.Errorf("a field a later revision added reads %v, %v, %v", n, err, readErr)
	}
	s, _ := fields.Field("s")
	if _, err := s.Record(""); err == nil || err.Error() != "'s' is not a record" {
		t.Errorf("a number where a record stands reads %v", err)
	}
	if _, err := r.Record("[]"); err == nil {
		t.Errorf("defaults that are no object were taken")
	}
}

func TestAListAndARecordAreWrittenInTheirOrderAndByTheirKeyBytes(t *testing.T) {
	if got := ListJSON([]uint8{}, func(v uint8) RawJSON { return ValueJSON(v) }); got != "[]" {
		t.Errorf("the empty list is written %s", got)
	}
	if got := ListJSON([]uint8{2, 1}, func(v uint8) RawJSON { return ValueJSON(v) }); got != "[2,1]" {
		t.Errorf("a list is written %s", got)
	}
	got := RecordJSON(Pair("b", 2), Pair("aa", 1), Pair("B", 3))
	if want := `{"B":3,"aa":1,"b":2}`; string(got) != want {
		t.Errorf("a record is written %s, want %s", got, want)
	}
	if got := PayloadJSON(Pair("x", RawJSON(`[1,{"a":null}]`))); got != `{"x":[1,{"a":null}]}` {
		t.Errorf("a value already spelled is written %s", got)
	}
}

func TestAValueAHostBuiltIsCheckedAtEveryDepthBeforeItIsRaised(t *testing.T) {
	tags := func(path string, items []string) error {
		return CheckList(path, items, 2, func(p string, s string) error { return CheckText(p, s, 3) })
	}
	if err := CheckList("items", [][]string{{"a"}, {"abc", "ab"}}, 3, tags); err != nil {
		t.Errorf("a value within every bound was refused: %v", err)
	}
	if err := CheckList("items", [][]string{{}, {}, {}, {}}, 3, tags); err == nil ||
		err.Error() != "'items' holds 4 elements, past the 3 its type declares" {
		t.Errorf("an outer list past its bound reads %v", err)
	}
	if err := CheckList("items", [][]string{{"a"}, {"a", "b", "c"}}, 3, tags); err == nil ||
		err.Error() != "'items/1' holds 3 elements, past the 2 its type declares" {
		t.Errorf("an inner list past its bound reads %v", err)
	}
	if err := CheckList("items", [][]string{{"a"}, {"a", "abcd"}}, 3, tags); err == nil ||
		err.Error() != "'items/1/1' is 4 bytes, past the 3 its type declares" {
		t.Errorf("a text past its bound reads %v", err)
	}
	if err := CheckBytes("blob", []byte{1, 2, 3, 4, 5}, 4); err == nil ||
		err.Error() != "'blob' is 5 bytes, past the 4 its type declares" {
		t.Errorf("a byte string past its bound reads %v", err)
	}
}
