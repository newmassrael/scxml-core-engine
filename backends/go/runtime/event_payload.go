// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import (
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"log"
	"math"
	"sort"
	"strconv"
	"strings"
	"unicode/utf8"
)

// Lifting an event's typed `_event.data` view out of the data it carries.
//
// NL→IR Item C1 Path A gives a schema'd event a typed payload that the
// natively lowered guards read. One producer fills it: the generated
// `<Machine>Raise<Event>` inject seam. Every other producer — `<send>` with
// `<param>`, namelist or `<content>`, an invoke forwarding an event either
// way, autoforward, BasicHTTP, mesh — fills `EventMetadata.Data`, the wire W3C
// SCXML 5.10 describes and §scxml-B-2-8-1 reads.
//
// Until this file the two never met, so the same guard answered differently
// depending on where its event came from, and a typed payload could not cross
// an invoke boundary at all. No data, a missing field, or a value of another
// type is a delivery outside its type, which the engine refuses whole where it
// takes the event: no transition is selected for it, and an error.execution
// that carries the refusal is taken in its place (docs/adr/0086, section 4.2).
//
// A field may be of any type of the language (docs/adr/0086): a list of any
// type, an optional value, a record, nested to any depth. Each is read as one
// PayloadValue at the path that names it, and three combinators walk the
// constructors that hold other values, each calling the reader of its part:
// ReadList, PayloadValue.Record and the optional package's Read. A value
// outside its type is refused before anything of it is kept, and the refusal
// names the path through every depth: `'items/2/tags/0' is 9 bytes, past the
// 8 its type declares`. A key a record's schema does not declare is stepped
// over.
//
// A refusal is a *PayloadRefusal: the sentence the delivery's error.execution
// carries, and beside it what a statechart reads as data (docs/adr/0087,
// section 2.4): a RefusalCode, the place as a strict RFC 6901 pointer
// (`/items/2/tags/0`, `~` written `~0` and `/` written `~1`, empty for the
// whole text) and the bound the value broke. A repeated name anywhere in the
// text, and a text holding U+0000, are refused, so every runtime's reader
// holds one set of texts (tests/json_text/payload_refusal.json).
//
// Cross-language siblings: `sce_runtime.event_payload` (Python),
// `SCE::EventPayload` (C++), `com.sce.runtime.EventPayload` (Kotlin).

// RefusalCode is the class of a refusal: the enum
// `sce:std/lift/refusal_code.scxml` declares, each code at the number that
// document gives it (docs/adr/0087, section 2.3).
type RefusalCode uint8

// The codes, in the order of their numbers.
const (
	// RefusalNotJSON: the text is empty, is not JSON, carries more than one
	// JSON value, or holds an object that repeats a name.
	RefusalNotJSON RefusalCode = iota + 1
	// RefusalNotObject: the text is one JSON value that is not an object.
	RefusalNotObject
	// RefusalTooDeep: a container is opened deeper than any a value of the
	// schema holds.
	RefusalTooDeep
	// RefusalMissing: a field of a record is absent and no revision default
	// fills it.
	RefusalMissing
	// RefusalNullNotOptional: `null` stands where the type is not optional.
	RefusalNullNotOptional
	// RefusalWrongKind: a value of another JSON kind.
	RefusalWrongKind
	// RefusalBadNumber: a number not in its type's spelling.
	RefusalBadNumber
	// RefusalOutOfRange: a whole number past its type's width, or a real past
	// a float32's range.
	RefusalOutOfRange
	// RefusalTooLong: a text or byte string past its bound.
	RefusalTooLong
	// RefusalTooMany: a list past its bound.
	RefusalTooMany
	// RefusalUnknownVariant: a name the enum does not declare.
	RefusalUnknownVariant
	// RefusalNotByteText: a byte string's text holding a character above
	// U+00FF.
	RefusalNotByteText
	// RefusalNulInText: a text holding U+0000.
	RefusalNulInText
	// RefusalTooFew: a list short of the lower count its type states
	// (docs/adr/0086, section 15).
	RefusalTooFew
)

// refusalCodeNames are the names the enum declares the codes by, indexed by
// number.
var refusalCodeNames = [...]string{
	RefusalNotJSON:         "notJson",
	RefusalNotObject:       "notObject",
	RefusalTooDeep:         "tooDeep",
	RefusalMissing:         "missing",
	RefusalNullNotOptional: "nullNotOptional",
	RefusalWrongKind:       "wrongKind",
	RefusalBadNumber:       "badNumber",
	RefusalOutOfRange:      "outOfRange",
	RefusalTooLong:         "tooLong",
	RefusalTooMany:         "tooMany",
	RefusalUnknownVariant:  "unknownVariant",
	RefusalNotByteText:     "notByteText",
	RefusalNulInText:       "nulInText",
	RefusalTooFew:          "tooFew",
}

// Name is the name the enum declares the code by; "" for a number it does not
// declare.
func (c RefusalCode) Name() string {
	if int(c) < len(refusalCodeNames) {
		return refusalCodeNames[c]
	}
	return ""
}

// RefusalCodeNamed is the code the enum declares by name, and whether it
// declares one.
func RefusalCodeNamed(name string) (RefusalCode, bool) {
	for code, declared := range refusalCodeNames {
		if declared != "" && declared == name {
			return RefusalCode(code), true
		}
	}
	return 0, false
}

// PayloadRefusal is why an event's data cannot be read as its schema's fields:
// the sentence Error answers, and beside it the code, the strict RFC 6901
// pointer to the place refused and, for a text, a byte string or a list past
// its bound, the bound it broke.
type PayloadRefusal struct {
	reason   string
	code     RefusalCode
	pointer  string
	bound    uint32
	hasBound bool
}

// Error is the sentence the delivery's error.execution carries.
func (r *PayloadRefusal) Error() string { return r.reason }

// Code is the class of the refusal.
func (r *PayloadRefusal) Code() RefusalCode { return r.code }

// Pointer is the place refused, as a strict RFC 6901 pointer: "" for the whole
// text.
func (r *PayloadRefusal) Pointer() string { return r.pointer }

// Bound is the bound the value broke, and whether the refusal names one.
func (r *PayloadRefusal) Bound() (uint32, bool) { return r.bound, r.hasBound }

// refuseAt is a refusal of class code at pointer, saying reason.
func refuseAt(code RefusalCode, pointer, reason string) *PayloadRefusal {
	return &PayloadRefusal{reason: reason, code: code, pointer: pointer}
}

// withBound is the refusal naming the bound the value broke; a bound no uint32
// holds is no bound a type declares, and is held at the largest one.
func (r *PayloadRefusal) withBound(bound int) *PayloadRefusal {
	r.hasBound = true
	if bound < 0 || uint64(bound) > math.MaxUint32 {
		r.bound = math.MaxUint32
	} else {
		r.bound = uint32(bound)
	}
	return r
}

// PointerBelow is token below the strict RFC 6901 pointer prefix ("" for the
// whole text): `/` and the token with `~` written `~0` and `/` written `~1`
// (RFC 6901, section 3).
func PointerBelow(prefix, token string) string {
	escaped := strings.NewReplacer("~", "~0", "/", "~1").Replace(token)
	return prefix + "/" + escaped
}

// PayloadFields is a decoded `_event.data`: the fields an event carries, each
// value still in the spelling it was written in, so a whole number stays exact
// until it is read at the width its schema declares.
type PayloadFields map[string]any

// LiftPayload decodes an event's data into the fields it names.
//
// The JSON read is §scxml-B-2-8-1's second rung, the same one the script
// engine takes for the same string.
func LiftPayload(data string) (PayloadFields, error) {
	// Only the whitespace RFC 8259 has: any other is a text that is not JSON.
	trimmed := strings.Trim(data, jsonWhitespace)
	if trimmed == "" {
		return nil, refuseAt(RefusalNotJSON, "", "the event carries no data")
	}
	decoded, err := decodeJSON(trimmed)
	if err != nil {
		return nil, refuseAt(RefusalNotJSON, "", "the event's data is not JSON: "+err.Error())
	}
	obj, ok := decoded.(map[string]any)
	if !ok {
		return nil, refuseAt(RefusalNotObject, "",
			"the event's data is a bare value, and this event's schema declares named fields")
	}
	return PayloadFields(obj), nil
}

// errMoreThanOneValue is why a text that holds one JSON value and then more is
// not one JSON value.
var errMoreThanOneValue = errors.New("it carries more than one JSON value")

// jsonWhitespace is the whitespace RFC 8259 has between tokens (section 2):
// space, tab, line feed and carriage return, and no other.
const jsonWhitespace = " \t\n\r"

// errHalfOfACharacter is why a text whose `\u` escape is half of a UTF-16 pair
// is not JSON every runtime reads alike: encoding/json reads a lone half as
// U+FFFD, which the text did not say.
var errHalfOfACharacter = errors.New("a \\u escape is half of a character")

// pairedEscapes reports a `\u` escape of text that names half of a UTF-16 pair
// with no other half beside it (RFC 8259, sections 7 and 8.2): a high half not
// followed at once by an escape of a low one, or a low half alone. Every other
// rule of a text's grammar is encoding/json's, which holds them.
func pairedEscapes(text string) error {
	inText := false
	for at := 0; at < len(text); at++ {
		switch c := text[at]; {
		case !inText:
			inText = c == '"'
		case c == '"':
			inText = false
		case c == '\\' && at+1 < len(text) && text[at+1] == 'u':
			code, ok := hex4(text, at+2)
			if !ok {
				// The grammar's to refuse.
				at++
				continue
			}
			switch {
			case code >= 0xD800 && code <= 0xDBFF:
				low, ok := uint16(0), false
				if at+7 < len(text) && text[at+6] == '\\' && text[at+7] == 'u' {
					low, ok = hex4(text, at+8)
				}
				if !ok || low < 0xDC00 || low > 0xDFFF {
					return errHalfOfACharacter
				}
				at += 11
			case code >= 0xDC00 && code <= 0xDFFF:
				return errHalfOfACharacter
			default:
				at += 5
			}
		case c == '\\':
			at++
		}
	}
	return nil
}

// hex4 is the number the four hex digits of text at at spell, and whether they
// are four hex digits.
func hex4(text string, at int) (uint16, bool) {
	if at+4 > len(text) {
		return 0, false
	}
	var code uint16
	for _, c := range []byte(text[at : at+4]) {
		var digit byte
		switch {
		case c >= '0' && c <= '9':
			digit = c - '0'
		case c >= 'a' && c <= 'f':
			digit = c - 'a' + 10
		case c >= 'A' && c <= 'F':
			digit = c - 'A' + 10
		default:
			return 0, false
		}
		code = code<<4 | uint16(digit)
	}
	return code, true
}

// decodeJSON reads exactly one JSON value, a whole number kept in its written
// spelling.
//
// An object whose names repeat has no single meaning: encoding/json keeps the
// last value, a reader that takes the first would read another, so it is
// refused, at any depth, as the other runtimes' readers refuse it
// (docs/adr/0087, section 2.3). The text is read token by token for that, since
// a map decoded whole no longer shows the name twice.
func decodeJSON(text string) (any, error) {
	if err := pairedEscapes(text); err != nil {
		return nil, err
	}
	dec := json.NewDecoder(strings.NewReader(text))
	// A whole number that does not fit a float64 must survive the decode, so
	// numbers are kept in their written spelling and converted per field.
	dec.UseNumber()
	decoded, err := decodeValue(dec)
	if err != nil {
		return nil, err
	}
	if _, err := dec.Token(); err != io.EOF {
		return nil, errMoreThanOneValue
	}
	return decoded, nil
}

// decodeValue reads the value whose first token dec reads next.
func decodeValue(dec *json.Decoder) (any, error) {
	token, err := dec.Token()
	if err != nil {
		return nil, err
	}
	delim, ok := token.(json.Delim)
	if !ok {
		// A text, a json.Number, a truth value or nil.
		return token, nil
	}
	switch delim {
	case '{':
		members := map[string]any{}
		for dec.More() {
			nameToken, err := dec.Token()
			if err != nil {
				return nil, err
			}
			name, ok := nameToken.(string)
			if !ok {
				return nil, errors.New("a field name was expected")
			}
			if _, seen := members[name]; seen {
				return nil, fmt.Errorf("the field %q appears twice", name)
			}
			value, err := decodeValue(dec)
			if err != nil {
				return nil, err
			}
			members[name] = value
		}
		if _, err := dec.Token(); err != nil {
			return nil, err
		}
		return members, nil
	case '[':
		items := []any{}
		for dec.More() {
			item, err := decodeValue(dec)
			if err != nil {
				return nil, err
			}
			items = append(items, item)
		}
		if _, err := dec.Token(); err != nil {
			return nil, err
		}
		return items, nil
	}
	return nil, fmt.Errorf("unexpected %q", rune(delim))
}

// LiftPayloadWithDefaults is LiftPayload for a schema a later revision of which
// added fields: defaults is the JSON object of those fields' defaults, which the
// generator spells from the schema, and a field the delivery lacks is read out
// of it instead (docs/SCE_ACCEPTED_SUBSET.md, "An event schema that grows").
//
// A delivery with no data at all lacks every field, so it is read as one that
// names none: a field of the first revision is then refused by name, and a
// schema all of whose fields came later reads its defaults. A field the delivery
// does carry is read as it came, and held to its type as any other value is.
func LiftPayloadWithDefaults(data, defaults string) (PayloadFields, error) {
	fields := PayloadFields{}
	if strings.Trim(data, jsonWhitespace) != "" {
		lifted, err := LiftPayload(data)
		if err != nil {
			return nil, err
		}
		fields = lifted
	}
	fallback, err := defaultsOf(defaults)
	if err != nil {
		return nil, err
	}
	for name, value := range fallback {
		if _, ok := fields[name]; !ok {
			fields[name] = value
		}
	}
	return fields, nil
}

// LiftText reads the text a `<sce:lift>` reads as the fields of a record of a
// schema (docs/adr/0087, section 2.1): depth is the deepest container a value
// of the schema holds, pathBound the room of the refusal record's `path` a
// tooDeep pointer is held to (LiftTooDeep), and defaults, for a schema a later
// revision of which added fields, the JSON object of their defaults ("" for
// none), which a text that lacks one reads in its place.
//
// Three refusals come before the fields are read, in this order: a text that is
// empty or only whitespace (notJson); a container opened deeper than depth,
// decided over the text's brackets alone before it is read as JSON (LiftTooDeep,
// tooDeep); and a text that is not one JSON object, as a delivery is refused
// (LiftPayload).
func LiftText(text string, depth, pathBound uint32, defaults string) (PayloadFields, error) {
	if strings.Trim(text, jsonWhitespace) == "" {
		return nil, refuseAt(RefusalNotJSON, "", "the text is empty")
	}
	if refusal := LiftTooDeep(text, depth, pathBound); refusal != nil {
		return nil, refusal
	}
	fields, err := LiftPayload(text)
	if err != nil {
		return nil, err
	}
	if defaults != "" {
		fallback, err := defaultsOf(defaults)
		if err != nil {
			return nil, err
		}
		for name, value := range fallback {
			if _, ok := fields[name]; !ok {
				fields[name] = value
			}
		}
	}
	return fields, nil
}

// LiftTooDeep is the refusal of a container text opens deeper than depth, the
// deepest a value of a lift's schema holds (docs/adr/0087, section 2.1, item
// 4), or nil when it opens none.
//
// Decided over the brackets alone, before the text is read as JSON, so that
// every runtime decides it alike and no reader descends past the type: a text
// is skipped from its opening quote to the next quote no backslash escapes,
// each `{` and `[` outside one opens a level and each `}` and `]` closes one,
// and the first opened past depth is refused, whatever the text holds after it.
// A text that never closes ends the walk with nothing refused, and the reading
// of the text as JSON refuses it.
//
// The pointer names the container refused (docs/adr/0087, section 2.4): for each
// container it stands in, outermost first, the element's index in an array (the
// `,` the scan met at that level since the array opened) or the member's name in
// an object (the last text closed at that level before a `:`, since the object
// opened or its last `,`), each an RFC 6901 token. It stops before the first
// place it cannot write: a member with no name before its colon, a name that is
// no text (no JSON string, not UTF-8, or holding U+0000), or a token that would
// take the pointer past pathBound UTF-8 bytes, the room of the refusal record's
// `path`. The bound is depth. The scan keeps one entry per open container, at
// most depth of them.
func LiftTooDeep(text string, depth, pathBound uint32) *PayloadRefusal {
	var open []liftScanLevel
	for at := 0; at < len(text); at++ {
		top := len(open) - 1
		switch text[at] {
		case '"':
			start := at
			at++
			for at < len(text) && text[at] != '"' {
				if text[at] == '\\' {
					at++
				}
				at++
			}
			if at >= len(text) {
				return nil
			}
			if top >= 0 {
				open[top].lastText = text[start : at+1]
			}
		case ':':
			if top >= 0 {
				open[top].member, open[top].lastText = open[top].lastText, ""
			}
		case ',':
			if top >= 0 {
				open[top].member, open[top].lastText = "", ""
				open[top].index++
			}
		case '{', '[':
			if uint32(len(open)) >= depth {
				return refuseAt(RefusalTooDeep, liftScanPointer(open, pathBound), fmt.Sprintf(
					"the text opens a container deeper than the %d levels a value of its schema holds",
					depth)).withBound(int(depth))
			}
			open = append(open, liftScanLevel{array: text[at] == '['})
		case '}', ']':
			if top >= 0 {
				open = open[:top]
			}
		}
	}
	return nil
}

// liftScanLevel is one container LiftTooDeep stands in: whether it is an array,
// the elements it has passed, the last text closed in it, and the member a `:`
// after that text named ("" for none).
type liftScanLevel struct {
	array            bool
	index            uint64
	lastText, member string
}

// liftScanPointer is the pointer to the place open stands at, as LiftTooDeep
// writes it.
func liftScanPointer(open []liftScanLevel, pathBound uint32) string {
	pointer := ""
	for _, level := range open {
		token := strconv.FormatUint(level.index, 10)
		if !level.array {
			if level.member == "" || !utf8.ValidString(level.member) {
				break
			}
			decoded, err := decodeJSON(level.member)
			name, ok := decoded.(string)
			if err != nil || !ok || strings.ContainsRune(name, 0) {
				break
			}
			token = name
		}
		below := PointerBelow(pointer, token)
		if len(below) > int(pathBound) {
			break
		}
		pointer = below
	}
	return pointer
}

// LiftField is the value of the field name of the record r, read by read: what a
// lift reads each field of its schema by.
func LiftField[T any](r PayloadRecord, name string, read func(PayloadValue) (T, error)) (T, error) {
	v, err := r.Field(name)
	if err != nil {
		var zero T
		return zero, err
	}
	return read(v)
}

// LiftRecord is what a `<sce:lift>` reads (docs/adr/0087, section 2.1): text as a
// record of the schema named schema, whose fields read reads out of the record
// it is handed, as a delivery of the schema is read; or the refusal of the first
// place that is not one, its code, pointer and bound the statechart keeps as a
// record. LiftText says what is refused before the fields are read. Each outcome
// is logged with the schema's name and, for a refusal, its code, pointer and
// bound; the text itself is not, since nothing bounds what a sender writes in it.
func LiftRecord[T any](text, schema string, depth, pathBound uint32, defaults string,
	read func(PayloadRecord) (T, error)) (T, *PayloadRefusal) {
	var zero T
	fields, err := LiftText(text, depth, pathBound, defaults)
	value := zero
	if err == nil {
		value, err = read(PayloadRecord{members: map[string]any(fields)})
	}
	if err == nil {
		log.Printf("[sce] <sce:lift>: the text is a record of %s", schema)
		return value, nil
	}
	var refusal *PayloadRefusal
	if !errors.As(err, &refusal) {
		// Every reader of this file refuses with a *PayloadRefusal; an error of
		// another kind is the text not being JSON at all.
		refusal = refuseAt(RefusalNotJSON, "", err.Error())
	}
	bound, hasBound := refusal.Bound()
	log.Printf("[sce] <sce:lift>: the text is no record of %s: %s at '%s', bound %d (%t) (%s)",
		schema, refusal.Code().Name(), refusal.Pointer(), bound, hasBound, refusal.Error())
	return zero, refusal
}

// defaultsOf is a schema's defaults, which the generator spells as one JSON
// object. A text that is none is the generator's, not a sender's, and is
// refused as no JSON object of the whole.
func defaultsOf(defaults string) (map[string]any, error) {
	decoded, err := decodeJSON(strings.TrimSpace(defaults))
	obj, ok := decoded.(map[string]any)
	if err != nil || !ok {
		return nil, refuseAt(RefusalNotJSON, "",
			"the schema's defaults are not a JSON object, which the generator never writes")
	}
	return obj, nil
}

// Field is the value of the field name, at the path its name is; a delivery
// that lacks it is refused by name.
func (obj PayloadFields) Field(name string) (PayloadValue, error) {
	return PayloadRecord{members: obj}.Field(name)
}

// PayloadUnknownVariant is the refusal of the text variant the payload's field
// name carries, which the enum alias does not declare a variant by: what a
// generated reader of an enum field answers once it has matched every name
// the enum declares.
func PayloadUnknownVariant(name, variant, alias string) error {
	return refuseAt(RefusalUnknownVariant, PointerBelow("", name),
		fmt.Sprintf("%q (%s) is not a variant of %s", name, variant, alias))
}

// PayloadRecord is the members of an object a record type is read from, at the
// path that names the record, and the defaults of the fields a later revision
// of its schema added.
type PayloadRecord struct {
	members  map[string]any
	defaults map[string]any
	prefix   string
	pointer  string
}

// Field is the value of the field name: the delivered one, else the default a
// later revision of the record's schema gave it, else a refusal naming the
// path. A member the schema does not declare is never asked for, so it is
// stepped over.
func (r PayloadRecord) Field(name string) (PayloadValue, error) {
	path := joinPath(r.prefix, name)
	pointer := PointerBelow(r.pointer, name)
	if v, ok := r.members[name]; ok {
		return PayloadValue{value: v, path: path, pointer: pointer}, nil
	}
	if v, ok := r.defaults[name]; ok {
		return PayloadValue{value: v, path: path, pointer: pointer}, nil
	}
	return PayloadValue{}, refuseAt(RefusalMissing, pointer,
		fmt.Sprintf("the event's data has no '%s'", path))
}

// joinPath is segment below prefix, as a JSON Pointer's reference tokens are
// joined.
func joinPath(prefix, segment string) string {
	if prefix == "" {
		return segment
	}
	return prefix + "/" + segment
}

// PayloadValue is one value of an event's data, at the path through every
// depth that names it: a field's name, then each index and field name below
// it, joined by `/`; and at the strict RFC 6901 pointer to the same place,
// which a refusal carries beside its sentence.
type PayloadValue struct {
	value   any
	path    string
	pointer string
}

// Path is the path that names the value.
func (v PayloadValue) Path() string { return v.path }

// Pointer is the strict RFC 6901 pointer to the value (`/items/2`).
func (v PayloadValue) Pointer() string { return v.pointer }

// IsNull reports whether the value is `null`, the one spelling of an absent
// value: what the optional package's Read asks.
func (v PayloadValue) IsNull() bool { return v.value == nil }

// Refuse is a refusal of the value, of class code, saying what of it after its
// path: `'items/1/level' (x) is not a variant of Level`.
func (v PayloadValue) Refuse(code RefusalCode, what string) error {
	return v.refuse(code, what)
}

// UnknownVariant is the refusal of the text name the value carries, which the
// enum alias does not declare a variant by.
func (v PayloadValue) UnknownVariant(name, alias string) error {
	return v.refuse(RefusalUnknownVariant, fmt.Sprintf("(%s) is not a variant of %s", name, alias))
}

func (v PayloadValue) refuse(code RefusalCode, what string) *PayloadRefusal {
	return refuseAt(code, v.pointer, fmt.Sprintf("'%s' %s", v.path, what))
}

// wrongKind is the refusal of `null` where the type is not optional, or the
// refusal otherwise of any other value of the wrong kind.
func (v PayloadValue) wrongKind(otherwise string) error {
	if v.IsNull() {
		return v.refuse(RefusalNullNotOptional, "is null, and its type is not optional")
	}
	return v.refuse(RefusalWrongKind, otherwise)
}

// spellsWhole reports whether text spells a whole number: digits, after a `-`
// or not.
func spellsWhole(text string) bool {
	digits := strings.TrimPrefix(text, "-")
	if digits == "" {
		return false
	}
	for i := 0; i < len(digits); i++ {
		if digits[i] < '0' || digits[i] > '9' {
			return false
		}
	}
	return true
}

// number is the value as the JSON number it was written as.
//
// ⚠ A truth value is not a number here. JSON spells both, and a schema that
// declared uint32 and received `true` has been handed something its own type
// says cannot occur.
func (v PayloadValue) number() (json.Number, error) {
	n, ok := v.value.(json.Number)
	if !ok {
		return "", v.wrongKind(fmt.Sprintf("is not a number (%v)", v.value))
	}
	return n, nil
}

// exactInADouble is the largest whole number every engine holds exactly as a
// number (2^53 - 1, ECMAScript's Number.MAX_SAFE_INTEGER), and so the largest a
// JSON number may carry for a 64-bit field.
const exactInADouble = 1<<53 - 1

// isDecimal reports whether text is a whole number written as digits and
// nothing else: no sign but `-` (and only where signed), no leading zero, no `-0`.
func isDecimal(text string, signed bool) bool {
	digits := text
	if strings.HasPrefix(text, "-") {
		if !signed {
			return false
		}
		digits = text[1:]
	}
	if digits == "" || (digits == "0" && text != "0") {
		return false
	}
	if digits != "0" && digits[0] == '0' {
		return false
	}
	for i := 0; i < len(digits); i++ {
		if digits[i] < '0' || digits[i] > '9' {
			return false
		}
	}
	return true
}

// wide is the digits of a 64-bit whole number: written as a text of its digits,
// or as a JSON number when it is exact in a double (|n| <= 2^53 - 1).
//
// A 64-bit integer crosses as a text because a reader that takes a JSON number
// through a double carries one past 2^53 with its low bits wrong and no error;
// a number past that is refused here on every engine, so the same payload gives
// the same field wherever it is read.
func (v PayloadValue) wide(signed bool) (string, error) {
	switch x := v.value.(type) {
	case string:
		if !isDecimal(x, signed) {
			return "", v.refuse(RefusalBadNumber,
				fmt.Sprintf("is not a whole number written as digits (%s)", x))
		}
		return x, nil
	case json.Number:
		// `-0` is no whole number, and a number below zero no spelling of an
		// unsigned one, as for every narrower width.
		if x.String() == "-0" || (!signed && strings.HasPrefix(x.String(), "-")) {
			return "", v.refuse(RefusalBadNumber,
				fmt.Sprintf("is not a whole number of its type (%s)", x.String()))
		}
		n, err := strconv.ParseInt(x.String(), 10, 64)
		if err != nil {
			return "", v.refuse(RefusalBadNumber, fmt.Sprintf("is not a whole number (%s)", x.String()))
		}
		if n > exactInADouble || n < -exactInADouble {
			return "", v.refuse(RefusalBadNumber, fmt.Sprintf(
				"is a number past 2^53 - 1, which is not exact everywhere (%s); a 64-bit integer "+
					"that large is written as a text of its digits", x.String()))
		}
		return x.String(), nil
	default:
		return "", v.wrongKind(fmt.Sprintf("is not a number (%v)", v.value))
	}
}

// pastWidth is the refusal of a number past the width its type declares.
func (v PayloadValue) pastWidth(text string) error {
	return v.refuse(RefusalOutOfRange, fmt.Sprintf("does not fit the width its schema declares (%s)", text))
}

// ValueSigned64 reads a 64-bit signed value (see wide).
func ValueSigned64(v PayloadValue) (int64, error) {
	text, err := v.wide(true)
	if err != nil {
		return 0, err
	}
	n, err := strconv.ParseInt(text, 10, 64)
	if err != nil {
		return 0, v.pastWidth(text)
	}
	return n, nil
}

// ValueUnsigned64 reads a 64-bit unsigned value (see wide).
func ValueUnsigned64(v PayloadValue) (uint64, error) {
	text, err := v.wide(false)
	if err != nil {
		return 0, err
	}
	n, err := strconv.ParseUint(text, 10, 64)
	if err != nil {
		return 0, v.pastWidth(text)
	}
	return n, nil
}

// ValueSigned reads a signed whole number at the width its type declares. A
// value the width cannot hold is refused rather than wrapped — silently
// truncating is how a guard on a boundary answers for a value the document
// never received.
func ValueSigned[T int8 | int16 | int32 | int64](v PayloadValue) (T, error) {
	n, err := v.number()
	if err != nil {
		return 0, err
	}
	// `-0` is a JSON number and no spelling of a whole number: every runtime
	// refuses it, as it refuses `"-0"` for a 64-bit integer.
	if n.String() == "-0" {
		return 0, v.refuse(RefusalBadNumber, "is not a whole number (-0)")
	}
	whole, err := n.Int64()
	if err != nil {
		// A whole number no int64 holds is past every signed width; any other
		// spelling is no whole number at all.
		code := RefusalBadNumber
		if spellsWhole(n.String()) {
			code = RefusalOutOfRange
		}
		return 0, v.refuse(code, fmt.Sprintf("is not a whole number (%s)", n.String()))
	}
	narrowed := T(whole)
	if int64(narrowed) != whole {
		return 0, v.pastWidth(n.String())
	}
	return narrowed, nil
}

// ValueUnsigned reads an unsigned whole number at its declared width.
func ValueUnsigned[T uint8 | uint16 | uint32 | uint64](v PayloadValue) (T, error) {
	n, err := v.number()
	if err != nil {
		return 0, err
	}
	whole, err := n.Int64()
	if err != nil || whole < 0 || n.String() == "-0" {
		// Below zero is no spelling of an unsigned number; a whole number no
		// int64 holds is past every width this reads.
		code := RefusalBadNumber
		if err != nil && spellsWhole(n.String()) && !strings.HasPrefix(n.String(), "-") {
			code = RefusalOutOfRange
		}
		return 0, v.refuse(code, fmt.Sprintf("is not a whole number at or above zero (%s)", n.String()))
	}
	narrowed := T(whole)
	if int64(narrowed) != whole {
		return 0, v.pastWidth(n.String())
	}
	return narrowed, nil
}

// ValueFloat reads a fractional value at its declared width.
//
// A JSON number is a double wherever it is read, so a float32 holds the binary32
// nearest that double — every engine rounds once, from the same double — and a
// number past the largest float32 does not fit, as a whole number past its width
// does not.
func ValueFloat[T float32 | float64](v PayloadValue) (T, error) {
	n, err := v.number()
	if err != nil {
		return 0, err
	}
	wide, err := n.Float64()
	if err != nil {
		// A number past every double's range is past every real width; any
		// other failure is a spelling no real is read from.
		code := RefusalBadNumber
		if errors.Is(err, strconv.ErrRange) {
			code = RefusalOutOfRange
		}
		return 0, v.refuse(code, fmt.Sprintf("is not a number (%s)", n.String()))
	}
	if _, single := any(T(0)).(float32); single && math.Abs(wide) > math.MaxFloat32 {
		return 0, v.pastWidth(n.String())
	}
	return T(wide), nil
}

// Bool reads a truth value.
func (v PayloadValue) Bool() (bool, error) {
	b, ok := v.value.(bool)
	if !ok {
		return false, v.wrongKind(fmt.Sprintf("is not a truth value (%v)", v.value))
	}
	return b, nil
}

// Name reads the text the value carries, held to no bound: what an enum's
// variant is named by, which its reader matches against the names its enum
// declares.
//
// A text holding U+0000 is refused: a runtime that holds a text NUL-terminated
// would end it there, so no runtime holds one, and every reader refuses the
// same texts (docs/adr/0087, section 2.3). A byte string may hold the byte
// zero (Bytes).
func (v PayloadValue) Name() (string, error) {
	s, ok := v.value.(string)
	if !ok {
		return "", v.wrongKind(fmt.Sprintf("is not a text (%v)", v.value))
	}
	if strings.ContainsRune(s, 0) {
		return "", v.refuse(RefusalNulInText, "carries U+0000, which no text holds")
	}
	return s, nil
}

// Text reads a text of at most bound UTF-8 bytes (docs/adr/0086, section 4.4):
// one past the bound is refused, at whatever depth it stands, and never cut.
func (v PayloadValue) Text(bound int) (string, error) {
	s, err := v.Name()
	if err != nil {
		return "", err
	}
	if len(s) > bound {
		return "", v.pastBytes(len(s), bound)
	}
	return s, nil
}

// Bytes reads a byte string of at most bound bytes.
//
// JSON has no byte string, so the wire carries the byte-exact Latin-1 text the
// inject seam writes, and this reads it back the same way: every one of the 256
// values is one character and back. Printable ASCII — what a bytes guard
// compares — is the same bytes under either reading.
func (v PayloadValue) Bytes(bound int) ([]byte, error) {
	s, ok := v.value.(string)
	if !ok {
		return nil, v.wrongKind(fmt.Sprintf("is not a byte string (%v)", v.value))
	}
	out := make([]byte, 0, len(s))
	for _, r := range s {
		if r > 0xFF {
			return nil, v.refuse(RefusalNotByteText,
				"carries a character above U+00FF, which no single byte spells")
		}
		out = append(out, byte(r))
	}
	if len(out) > bound {
		return nil, v.pastBytes(len(out), bound)
	}
	return out, nil
}

func (v PayloadValue) pastBytes(held, bound int) error {
	return v.refuse(RefusalTooLong,
		fmt.Sprintf("is %d bytes, past the %d its type declares", held, bound)).withBound(bound)
}

// ReadList reads a value of `list<T>[bound]`: an array of at most bound
// elements, each read by read at the path of its index, in order.
//
// The bound is held before any element is read: an array longer than bound is
// refused whole, and no element of it is decoded (ADR 0077, section 2).
func ReadList[T any](v PayloadValue, bound int, read func(PayloadValue) (T, error)) ([]T, error) {
	return ReadListWithin(v, 0, bound, read)
}

// ReadListWithin reads a value of `list<T>[least..bound]`: an array of at least
// least and at most bound elements (docs/adr/0086, section 15), each read by
// read at the path of its index, in order.
//
// Both counts are held before any element is read: an array longer than bound
// is refused as RefusalTooMany, one shorter than least as RefusalTooFew naming
// least, and no element of either is decoded.
func ReadListWithin[T any](v PayloadValue, least, bound int, read func(PayloadValue) (T, error)) ([]T, error) {
	items, ok := v.value.([]any)
	if !ok {
		return nil, v.wrongKind("is not a list")
	}
	if len(items) > bound {
		return nil, v.refuse(RefusalTooMany,
			fmt.Sprintf("holds %d elements, past the %d its type declares", len(items), bound)).withBound(bound)
	}
	if len(items) < least {
		return nil, v.refuse(RefusalTooFew, countShort(len(items), least)).withBound(least)
	}
	out := make([]T, 0, len(items))
	for index, item := range items {
		token := strconv.Itoa(index)
		element, err := read(PayloadValue{
			value:   item,
			path:    joinPath(v.path, token),
			pointer: PointerBelow(v.pointer, token),
		})
		if err != nil {
			return nil, err
		}
		out = append(out, element)
	}
	return out, nil
}

// Record reads a value of a record type: an object whose fields the returned
// record reads. defaults, for a record whose schema a later revision added
// fields to, is the JSON object of their defaults, which the generator spells
// from that schema, and a field the value lacks is read out of it
// (docs/adr/0086, section 10.3); "" for a schema with none.
func (v PayloadValue) Record(defaults string) (PayloadRecord, error) {
	members, ok := v.value.(map[string]any)
	if !ok {
		return PayloadRecord{}, v.wrongKind("is not a record")
	}
	record := PayloadRecord{members: members, prefix: v.path, pointer: v.pointer}
	if defaults != "" {
		fallback, err := defaultsOf(defaults)
		if err != nil {
			return PayloadRecord{}, err
		}
		record.defaults = fallback
	}
	return record, nil
}

// ReadUnion reads a value of a union type (docs/adr/0092, section 4): one
// object holding its tag under the key tag and the fields of the case the tag
// names beside it.
//
// The tag is read first, by readTag, the reader of the tag's enum, wherever it
// stands in the object; readCase is then handed the case the tag named and this
// same value, and reads the case's record from it with PayloadValue.Record (an
// empty case reads nothing). The refusals come in one order on every runtime: a
// value that is not an object; the tag absent (RefusalMissing at the tag's
// pointer), then whatever the enum's reader refuses of it (null, not a text, a
// name the enum does not declare); then the case's record. A key the case does
// not declare is never asked for, so it is stepped over, as a record steps over
// one, and a text that carries another case's keys, or null under them, reads as
// the case its tag names.
func ReadUnion[E, T any](v PayloadValue, tag string, readTag func(PayloadValue) (E, error),
	readCase func(E, PayloadValue) (T, error)) (T, error) {
	var none T
	members, ok := v.value.(map[string]any)
	if !ok {
		return none, v.wrongKind("is not a union")
	}
	tagValue, err := PayloadRecord{members: members, prefix: v.path, pointer: v.pointer}.Field(tag)
	if err != nil {
		return none, err
	}
	chosen, err := readTag(tagValue)
	if err != nil {
		return none, err
	}
	return readCase(chosen, v)
}

// field is the value name carries, or a refusal naming what is missing.
func field(obj PayloadFields, name string) (PayloadValue, error) {
	return obj.Field(name)
}

// PayloadSigned64 reads a 64-bit signed field (ValueSigned64).
func PayloadSigned64(obj PayloadFields, name string) (int64, error) {
	v, err := field(obj, name)
	if err != nil {
		return 0, err
	}
	return ValueSigned64(v)
}

// PayloadUnsigned64 reads a 64-bit unsigned field (ValueUnsigned64).
func PayloadUnsigned64(obj PayloadFields, name string) (uint64, error) {
	v, err := field(obj, name)
	if err != nil {
		return 0, err
	}
	return ValueUnsigned64(v)
}

// Int64PayloadText is the text of the digits a 64-bit signed integer crosses as.
func Int64PayloadText(v int64) string { return strconv.FormatInt(v, 10) }

// Uint64PayloadText is the text of the digits a 64-bit unsigned integer crosses as.
func Uint64PayloadText(v uint64) string { return strconv.FormatUint(v, 10) }

// PayloadSigned reads a signed whole number field at the width its schema
// declared (ValueSigned).
func PayloadSigned[T int8 | int16 | int32 | int64](obj PayloadFields, name string) (T, error) {
	v, err := field(obj, name)
	if err != nil {
		return 0, err
	}
	return ValueSigned[T](v)
}

// PayloadUnsigned reads an unsigned whole number field at its declared width
// (ValueUnsigned).
func PayloadUnsigned[T uint8 | uint16 | uint32 | uint64](obj PayloadFields, name string) (T, error) {
	v, err := field(obj, name)
	if err != nil {
		return 0, err
	}
	return ValueUnsigned[T](v)
}

// PayloadFloat reads a fractional field at its declared width (ValueFloat).
func PayloadFloat[T float32 | float64](obj PayloadFields, name string) (T, error) {
	v, err := field(obj, name)
	if err != nil {
		return 0, err
	}
	return ValueFloat[T](v)
}

// PayloadBool reads a truth-value field.
func PayloadBool(obj PayloadFields, name string) (bool, error) {
	v, err := field(obj, name)
	if err != nil {
		return false, err
	}
	return v.Bool()
}

// PayloadString reads the text a field carries, held to no bound: what an enum
// field's variant is named by, which its reader matches against the names its
// enum declares. A text field is read by PayloadText, which holds its bound.
func PayloadString(obj PayloadFields, name string) (string, error) {
	v, err := field(obj, name)
	if err != nil {
		return "", err
	}
	return v.Name()
}

// PayloadText reads a text field of at most bound UTF-8 bytes (docs/adr/0086,
// section 4.4): the top level of a payload is held to its bound by
// PayloadValue.Text, the reader a text at any depth is held by.
func PayloadText(obj PayloadFields, name string, bound int) (string, error) {
	v, err := field(obj, name)
	if err != nil {
		return "", err
	}
	return v.Text(bound)
}

// PayloadBytes reads a byte-string field of at most bound bytes (see
// PayloadValue.Bytes, the reader a byte string at any depth is held by).
func PayloadBytes(obj PayloadFields, name string, bound int) ([]byte, error) {
	v, err := field(obj, name)
	if err != nil {
		return nil, err
	}
	return v.Bytes(bound)
}

// PayloadPair is one field of an inject seam's payload, in schema order.
type PayloadPair struct {
	name  string
	value any
}

// Pair names one payload field for PayloadJSON.
func Pair(name string, value any) PayloadPair {
	return PayloadPair{name: name, value: value}
}

// RawJSON is a value already spelled as JSON text, which PayloadJSON and
// RecordJSON write as it is: a list or a record its writer spelled.
type RawJSON string

// PayloadJSON is the wire spelling of the fields an inject seam was given, so
// that the script engine binds `_event.data` to the same values the typed
// carrier holds. A byte string is written as its Latin-1 text (see
// PayloadBytes).
//
// The fields are written in the order they are given, which is the order the
// schema declares them — the order every other engine writes the same
// payload in. A map would lose it: this took one until 2026-09-28 and wrote
// the fields in name order, so the same event's `_event.data` differed by
// engine. Names and texts are written with EscapeJSONString, the form every
// engine writes a string in; encoding/json would escape `<`, `>`, `&` and
// U+2028 as well and replace a byte that is not UTF-8, which no other engine
// does.
func PayloadJSON(fields ...PayloadPair) string {
	var b strings.Builder
	writeMembers(&b, fields)
	return b.String()
}

// RecordJSON is the JSON spelling of one record (docs/adr/0086, section 3.1):
// each pair is a field's id and its value, and the object writes them
// ascending by the id's UTF-8 bytes, whatever order they are given in
// (ARCHITECTURE.md, "JSON Object Key Order", at every depth).
func RecordJSON(members ...PayloadPair) RawJSON {
	sorted := append([]PayloadPair(nil), members...)
	sort.SliceStable(sorted, func(i, j int) bool { return sorted[i].name < sorted[j].name })
	var b strings.Builder
	writeMembers(&b, sorted)
	return RawJSON(b.String())
}

// UnionJSON is the JSON spelling of one union value (docs/adr/0092, section
// 2.1): the tag under the key tag, its value variant, the case's name as its enum
// spells it, beside members, the fields of the case's record (none for an empty
// case). The object writes the tag among the fields, every key ascending by its
// UTF-8 bytes, as RecordJSON writes a record.
func UnionJSON(tag, variant string, members ...PayloadPair) RawJSON {
	all := make([]PayloadPair, 0, len(members)+1)
	all = append(all, members...)
	return RecordJSON(append(all, Pair(tag, variant))...)
}

// ListJSON is the JSON spelling of one list: an array of what write spells each
// element as, in the list's order. The empty list is `[]`.
func ListJSON[T any](items []T, write func(T) RawJSON) RawJSON {
	var b strings.Builder
	b.WriteByte('[')
	for i, item := range items {
		if i > 0 {
			b.WriteByte(',')
		}
		b.WriteString(string(write(item)))
	}
	b.WriteByte(']')
	return RawJSON(b.String())
}

// ValueJSON is the JSON spelling PayloadJSON gives one value: what a list's
// element of a scalar type is written as.
func ValueJSON(value any) RawJSON {
	var b strings.Builder
	writeValue(&b, value)
	return RawJSON(b.String())
}

// writeMembers writes fields as one object, in the order given.
func writeMembers(b *strings.Builder, fields []PayloadPair) {
	b.WriteByte('{')
	for i, field := range fields {
		if i > 0 {
			b.WriteByte(',')
		}
		b.WriteByte('"')
		b.WriteString(EscapeJSONString(field.name))
		b.WriteString(`":`)
		writeValue(b, field.value)
	}
	b.WriteByte('}')
}

// writeValue writes one value as JSON: a text with EscapeJSONString, a float
// with NumberText, a value already spelled as it is, and the rest as
// encoding/json writes it.
func writeValue(b *strings.Builder, value any) {
	switch v := value.(type) {
	case RawJSON:
		b.WriteString(string(v))
		return
	case string:
		b.WriteByte('"')
		b.WriteString(EscapeJSONString(v))
		b.WriteByte('"')
		return
	case float64:
		// encoding/json spells a float as ES6 does but keeps the sign of a
		// negative zero; the one spelling is [NumberText].
		if !math.IsNaN(v) && !math.IsInf(v, 0) {
			b.WriteString(NumberText(v))
			return
		}
	}
	encoded, err := json.Marshal(value)
	if err != nil {
		// Every value here comes from a schema's own field types, all of
		// which marshal; there is no input the caller could give that lands
		// here.
		return
	}
	b.Write(encoded)
}

// BytesAsPayloadText is the Latin-1 spelling of a byte string, for the inject
// seam's `Data` beside the typed carrier.
func BytesAsPayloadText(b []byte) string {
	runes := make([]rune, 0, len(b))
	for _, c := range b {
		runes = append(runes, rune(c))
	}
	return string(runes)
}

// CheckText holds a text a host built to the bound its type declares, before a
// generated raise encodes it (docs/adr/0086, section 5.4): Go has no checked
// constructor on a plain struct, so the raise runs the check its composition
// generates, one call per constructor, and refuses the event whole.
func CheckText(path, value string, bound int) error {
	if len(value) > bound {
		return fmt.Errorf("'%s' is %d bytes, past the %d its type declares", path, len(value), bound)
	}
	return nil
}

// CheckBytes holds a byte string a host built to its bound (see CheckText).
func CheckBytes(path string, value []byte, bound int) error {
	if len(value) > bound {
		return fmt.Errorf("'%s' is %d bytes, past the %d its type declares", path, len(value), bound)
	}
	return nil
}

// CheckList holds a list a host built to its bound, and each element to its
// own type through each, at the path of its index (see CheckText).
func CheckList[T any](path string, items []T, bound int, each func(string, T) error) error {
	return CheckListWithin(path, items, 0, bound, each)
}

// countShort is why a list of held elements is no value of a type whose lower
// count is least, in the words the reader and the check both use.
func countShort(held, least int) string {
	return fmt.Sprintf("holds %d elements, fewer than the %d its type declares", held, least)
}

// CheckListWithin holds a list a host or a machine built to its lower count and
// its bound (docs/adr/0086, section 15.6), and each element to its own type
// through each, at the path of its index (see CheckText): a list is built from
// empty an element at a time, so its lower count is held where it is written.
func CheckListWithin[T any](path string, items []T, least, bound int, each func(string, T) error) error {
	if len(items) > bound {
		return fmt.Errorf("'%s' holds %d elements, past the %d its type declares", path, len(items), bound)
	}
	if len(items) < least {
		return fmt.Errorf("'%s' %s", path, countShort(len(items), least))
	}
	for index, item := range items {
		if err := each(joinPath(path, strconv.Itoa(index)), item); err != nil {
			return err
		}
	}
	return nil
}
