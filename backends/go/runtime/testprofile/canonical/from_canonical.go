// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package canonical

import (
	"fmt"
	"strconv"
	"strings"

	sce "github.com/newmassrael/sce-go-runtime"
)

// A value read back from the canonical form of the test log
// (docs/SCE_TEST_PROFILE.md §12.5; docs/SCE_TTCN3_LANE.md §10.3): the inverse
// of the writers of this package, which a recorded input trace's values are
// read with (`schemas/sce-test-trace.v1.schema.json`). A value is read from the
// JSON value model of the runtime's JSON reader (sce.ParseJSON): a string, a
// bool, a sce.JSONNumber, a []any, a *sce.JSONObject or nil.
//
// Each reader takes one form and accepts its one spelling, so a value read
// here and written back by the writers is the same bytes:
//
//   - an integer is the text of its decimal digits, with no sign but a minus,
//     no leading zero and no `-0`, within the range of the type it is read as;
//   - a `bool` is a truth value;
//   - a value of an enum is the name its document declares for the variant,
//     or, for an enum of an open set, the text of a number no variant
//     declares;
//   - a text is a text of at most its bound in UTF-8 bytes;
//   - a byte string is its octets in upper-case hex, two digits each;
//   - a record is an object of its fields by their declared ids, an absent
//     field omitted, no field it does not declare;
//   - a list is an array of its elements within its bounds, an absent element
//     of a list of `optional<T>` null.
//
// A value that does not belong to its type is refused with a *Refusal, naming
// where it stands (`what`) and why, never converted. Which form a value takes
// is decided by the types its document declares and not by its Go type, since
// a byte string and a list of `uint8` are one Go type: the generated reader of
// a record (`Read<Record>` in the unit of a test configuration) calls these by
// the walk its canonical writer is spelled by
// (`forge::generator::test_template_walk::canonical_form`). Go lowers no union,
// so no form of one is read here.
//
// A JSON object names each member once, which sce.ParseJSON holds a text to and
// a *sce.JSONObject cannot break, so no reader here refuses a field given
// twice, as the Rust reader of its list of members must.
//
// Cross-language siblings: `sce_rust_runtime::test::from_canonical` (Rust),
// whose refusals these name in the same words, `SCE::Test::FromCanonical`
// (C++), `com.sce.runtime.test.FromCanonical` (Kotlin).

// Refusal is why a value is not read from its canonical form: the caller's
// input, refused by name before anything of the run happens.
type Refusal struct {
	// Reason says where the value stands and why.
	Reason string
}

func (r *Refusal) Error() string {
	return r.Reason
}

// refused is the refusal of the value at what, for why.
func refused(what, why string) *Refusal {
	return &Refusal{Reason: what + ": " + why}
}

// absentField is what Member answers for a field its record's object does not
// name: no JSON value, so no reader takes it for one.
type absentField struct{}

// kind is the kind of a JSON value, as a refusal names it.
func kind(value any) string {
	switch value.(type) {
	case nil:
		return "null"
	case bool:
		return "a truth value"
	case sce.JSONNumber:
		return "a number"
	case string:
		return "a text"
	case []any:
		return "an array"
	case *sce.JSONObject:
		return "an object"
	}
	return "a value of no JSON kind"
}

// notA is the refusal of value, at what, which is not of the form form. A
// field Member found absent is refused as absent, whatever its form: its type
// is not optional, or Member would not have been asked for it.
func notA(value any, what, form string) *Refusal {
	if _, absent := value.(absentField); absent {
		return refused(what, "the field is absent, and its type is not optional")
	}
	return refused(what, kind(value)+" is not "+form)
}

// greatest128 is the digits of the greatest 128-bit integer.
const greatest128 = "170141183460469231731687303715884105727"

// least128 is the digits of the magnitude of the least 128-bit integer, one
// more than the greatest's.
const least128 = "170141183460469231731687303715884105728"

// DecimalText reports whether text spells an integer as the canonical form
// writes one: an optional minus and decimal digits with no leading zero, never
// `-0`, within the range of a 128-bit integer, which the Rust reader of the
// same text holds it to. A number past that range is no such text, and one
// within it but past the type it is read as is past the type's range.
func DecimalText(text string) bool {
	magnitude, negative := strings.CutPrefix(text, "-")
	if magnitude == "" || (magnitude[0] == '0' && (len(magnitude) > 1 || negative)) {
		return false
	}
	for i := 0; i < len(magnitude); i++ {
		if magnitude[i] < '0' || magnitude[i] > '9' {
			return false
		}
	}
	// Two texts of digits of one length, neither with a leading zero, are in
	// the order of their values.
	bound := greatest128
	if negative {
		bound = least128
	}
	return len(magnitude) < len(bound) || (len(magnitude) == len(bound) && magnitude <= bound)
}

// integerOf is the value of the integer type N that digits, a text DecimalText
// holds, spells, and false where it is past the range of N.
func integerOf[N Integral](digits string) (N, bool) {
	if strings.HasPrefix(digits, "-") {
		wide, err := strconv.ParseInt(digits, 10, 64)
		if err != nil {
			return 0, false
		}
		number := N(wide)
		// An unsigned N wraps a negative number to one that is not, and a
		// narrow one cuts it to another number.
		if int64(number) != wide || number >= 0 {
			return 0, false
		}
		return number, true
	}
	wide, err := strconv.ParseUint(digits, 10, 64)
	if err != nil {
		return 0, false
	}
	number := N(wide)
	// A signed N of 64 bits wraps a number past its greatest to a negative
	// one, and a narrow one cuts it to another number.
	if uint64(number) != wide || number < 0 {
		return 0, false
	}
	return number, true
}

// ReadInteger is an integer of the type N: the text of its decimal digits.
func ReadInteger[N Integral](value any, what string) (N, error) {
	digits, isText := value.(string)
	if !isText {
		return 0, notA(value, what, "an integer, which is the text of its decimal digits")
	}
	if !DecimalText(digits) {
		return 0, refused(what, fmt.Sprintf("'%s' is not the text of an integer's decimal digits", digits))
	}
	number, fits := integerOf[N](digits)
	if !fits {
		return 0, refused(what, digits+" is past the range of its type")
	}
	return number, nil
}

// ReadBoolean is a `bool`.
func ReadBoolean(value any, what string) (bool, error) {
	held, isBool := value.(bool)
	if !isBool {
		return false, notA(value, what, "a truth value")
	}
	return held, nil
}

// ReadText is a text of at most max bytes of UTF-8, the bound its type
// declares.
func ReadText(value any, max int, what string) (string, error) {
	text, isText := value.(string)
	if !isText {
		return "", notA(value, what, "a text")
	}
	if len(text) > max {
		return "", refused(what, fmt.Sprintf("a text of %d bytes is past the %d its type holds", len(text), max))
	}
	return text, nil
}

// hexDigit is the value of one upper-case hex digit, and false for any other
// byte.
func hexDigit(digit byte) (byte, bool) {
	switch {
	case digit >= '0' && digit <= '9':
		return digit - '0', true
	case digit >= 'A' && digit <= 'F':
		return digit - 'A' + 10, true
	}
	return 0, false
}

// octetsOfUpperHex is the octets text spells in upper-case hex, two digits
// each, and false where it spells none.
func octetsOfUpperHex(text string) ([]byte, bool) {
	if len(text)%2 != 0 {
		return nil, false
	}
	octets := make([]byte, 0, len(text)/2)
	for i := 0; i < len(text); i += 2 {
		high, isHigh := hexDigit(text[i])
		low, isLow := hexDigit(text[i+1])
		if !isHigh || !isLow {
			return nil, false
		}
		octets = append(octets, high<<4|low)
	}
	return octets, true
}

// ReadBytes is a byte string of at most max octets: the text of its octets in
// upper-case hex, two digits each.
func ReadBytes(value any, max int, what string) ([]byte, error) {
	hex, isText := value.(string)
	if !isText {
		return nil, notA(value, what, "a byte string, which is a text of hex")
	}
	octets, spelled := octetsOfUpperHex(hex)
	if !spelled {
		return nil, refused(what, fmt.Sprintf("'%s' is not octets in upper-case hex, two digits each", hex))
	}
	if len(octets) > max {
		return nil, refused(what,
			fmt.Sprintf("a byte string of %d octets is past the %d its type holds", len(octets), max))
	}
	return octets, nil
}

// Variant is a variant of an enum as a reader reads it: the name its document
// declares and its value.
type Variant[E comparable] struct {
	// Name is the variant's declared name.
	Name string
	// Value is the variant.
	Value E
}

// declaredBy is the variant of declared whose name is name, and false where
// none is.
func declaredBy[E comparable](declared []Variant[E], name string) (E, bool) {
	for _, variant := range declared {
		if variant.Name == name {
			return variant.Value, true
		}
	}
	var none E
	return none, false
}

// ReadEnumerated is a value of an enum of a closed set: the variant of
// declared whose name the text is.
func ReadEnumerated[E comparable](value any, declared []Variant[E], what string) (E, error) {
	name, isText := value.(string)
	if !isText {
		var none E
		return none, notA(value, what, "a value of an enum, which is its name")
	}
	variant, named := declaredBy(declared, name)
	if !named {
		return variant, refused(what, fmt.Sprintf("'%s' names no variant of its enum", name))
	}
	return variant, nil
}

// ReadOpenEnumerated is a value of an enum of an open set
// (`sce:strict-variants="false"`): the variant of declared whose name the text
// is, or the value fromUnderlying makes of the number of the enum's underlying
// type U the text spells, where no variant declares that number. A declared
// number is written by its variant's name, so its digits are refused as the
// second spelling of one value.
func ReadOpenEnumerated[E comparable, U Integral](
	value any,
	declared []Variant[E],
	fromUnderlying func(U) E,
	what string,
) (E, error) {
	var none E
	name, isText := value.(string)
	if !isText {
		return none, notA(value, what, "a value of an enum, which is its name")
	}
	if variant, named := declaredBy(declared, name); named {
		return variant, nil
	}
	if !DecimalText(name) {
		return none, refused(what, fmt.Sprintf("'%s' names no variant of its enum, and is no number", name))
	}
	raw, fits := integerOf[U](name)
	if !fits {
		return none, refused(what, name+" is past the range of its enum")
	}
	held := fromUnderlying(raw)
	for _, variant := range declared {
		if variant.Value == held {
			return none, refused(what,
				name+" is the number of a declared variant, which is written by its name")
		}
	}
	return held, nil
}

// ReadRecord is a record whose fields are fields: an object, each member of
// which names one of them. It answers the object, whose fields Member and the
// object's own Get read.
func ReadRecord(value any, fields []string, what string) (*sce.JSONObject, error) {
	object, isObject := value.(*sce.JSONObject)
	if !isObject {
		return nil, notA(value, what, "a record, which is an object of its fields")
	}
	for _, name := range object.Keys() {
		declared := false
		for _, field := range fields {
			if field == name {
				declared = true
				break
			}
		}
		if !declared {
			return nil, refused(what, fmt.Sprintf("'%s' is no field of its record", name))
		}
	}
	return object, nil
}

// Member is the field id of the record record, whose type is not optional, so
// it is present: its value, which may be JSON null, which the reader of its
// type refuses. Where the object does not name it, every reader refuses what
// Member answers as a field that is absent, at the place the reader is told
// (`<record>.<id>`). A field whose type is optional is read with the object's
// Get, which says whether it is there, since the canonical form omits it where
// it is absent.
func Member(record *sce.JSONObject, id string) any {
	if value, present := record.Get(id); present {
		return value
	}
	return absentField{}
}

// ReadElements is the elements of a list of at least min and at most max
// elements.
func ReadElements(value any, min, max int, what string) ([]any, error) {
	items, isArray := value.([]any)
	if !isArray {
		return nil, notA(value, what, "a list, which is an array")
	}
	if len(items) < min || len(items) > max {
		return nil, refused(what,
			fmt.Sprintf("a list of %d elements is outside its bounds, %d to %d", len(items), min, max))
	}
	return items, nil
}
