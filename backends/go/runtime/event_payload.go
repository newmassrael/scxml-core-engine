// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import (
	"encoding/json"
	"fmt"
	"math"
	"sort"
	"strconv"
	"strings"
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
// an invoke boundary at all. What the lift refuses is what the SCRIPT ENGINE
// already refuses for the same guard, measured on the same document: no data,
// a missing field, or a value of another type each give error.execution and a
// guard that does not fire. A native lowering that answered differently would
// make the optimisation observable, which is the one thing it may not be.
//
// A field may be of any type of the language (docs/adr/0086): a list of any
// type, an optional value, a record, nested to any depth. Each is read as one
// PayloadValue at the path that names it, and three combinators walk the
// constructors that hold other values, each calling the reader of its part:
// ReadList, PayloadValue.Record and the optional package's Read. A value
// outside its type is refused before anything of it is kept, and the refusal
// names the path through every depth as a JSON Pointer (RFC 6901):
// `'items/2/tags/0' is 9 bytes, past the 8 its type declares`. A key a
// record's schema does not declare is stepped over.
//
// Cross-language siblings: `sce_runtime.event_payload` (Python),
// `SCE::EventPayload` (C++), `com.sce.runtime.EventPayload` (Kotlin).

// PayloadFields is a decoded `_event.data`: the fields an event carries, each
// value still in the spelling it was written in, so a whole number stays exact
// until it is read at the width its schema declares.
type PayloadFields map[string]any

// LiftPayload decodes an event's data into the fields it names.
//
// The JSON read is §scxml-B-2-8-1's second rung, the same one the script
// engine takes for the same string.
func LiftPayload(data string) (PayloadFields, error) {
	trimmed := strings.TrimSpace(data)
	if trimmed == "" {
		return nil, fmt.Errorf("the event carries no data")
	}
	decoded, err := decodeJSON(trimmed)
	if err != nil {
		return nil, fmt.Errorf("the event's data is not JSON: %w", err)
	}
	obj, ok := decoded.(map[string]any)
	if !ok {
		return nil, fmt.Errorf(
			"the event's data is a bare value, and this event's schema declares named fields")
	}
	return PayloadFields(obj), nil
}

// decodeJSON reads one JSON value, a whole number kept in its written spelling.
func decodeJSON(text string) (any, error) {
	dec := json.NewDecoder(strings.NewReader(text))
	// A whole number that does not fit a float64 must survive the decode, so
	// numbers are kept in their written spelling and converted per field.
	dec.UseNumber()
	var decoded any
	if err := dec.Decode(&decoded); err != nil {
		return nil, err
	}
	return decoded, nil
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
	if strings.TrimSpace(data) != "" {
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

// defaultsOf is a schema's defaults, which the generator spells as one JSON
// object.
func defaultsOf(defaults string) (map[string]any, error) {
	decoded, err := decodeJSON(strings.TrimSpace(defaults))
	obj, ok := decoded.(map[string]any)
	if err != nil || !ok {
		return nil, fmt.Errorf(
			"the schema's defaults are not a JSON object, which the generator never writes")
	}
	return obj, nil
}

// Field is the value of the field name, at the path its name is; a delivery
// that lacks it is refused by name.
func (obj PayloadFields) Field(name string) (PayloadValue, error) {
	return PayloadRecord{members: obj}.Field(name)
}

// PayloadRecord is the members of an object a record type is read from, at the
// path that names the record, and the defaults of the fields a later revision
// of its schema added.
type PayloadRecord struct {
	members  map[string]any
	defaults map[string]any
	prefix   string
}

// Field is the value of the field name: the delivered one, else the default a
// later revision of the record's schema gave it, else a refusal naming the
// path. A member the schema does not declare is never asked for, so it is
// stepped over.
func (r PayloadRecord) Field(name string) (PayloadValue, error) {
	path := joinPath(r.prefix, name)
	if v, ok := r.members[name]; ok {
		return PayloadValue{value: v, path: path}, nil
	}
	if v, ok := r.defaults[name]; ok {
		return PayloadValue{value: v, path: path}, nil
	}
	return PayloadValue{}, fmt.Errorf("the event's data has no '%s'", path)
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
// it, joined by `/`.
type PayloadValue struct {
	value any
	path  string
}

// Path is the path that names the value.
func (v PayloadValue) Path() string { return v.path }

// IsNull reports whether the value is `null`, the one spelling of an absent
// value: what the optional package's Read asks.
func (v PayloadValue) IsNull() bool { return v.value == nil }

// Refuse is a refusal of the value, saying what of it after its path:
// `'items/1/level' (x) is not a variant of Level`.
func (v PayloadValue) Refuse(what string) error {
	return fmt.Errorf("'%s' %s", v.path, what)
}

// wrongKind is the refusal of `null` where the type is not optional, or the
// refusal otherwise of any other value of the wrong kind.
func (v PayloadValue) wrongKind(otherwise string) error {
	if v.IsNull() {
		return v.Refuse("is null, and its type is not optional")
	}
	return v.Refuse(otherwise)
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
			return "", v.Refuse(fmt.Sprintf("is not a whole number written as digits (%s)", x))
		}
		return x, nil
	case json.Number:
		n, err := strconv.ParseInt(x.String(), 10, 64)
		if err != nil {
			return "", v.Refuse(fmt.Sprintf("is not a whole number (%s)", x.String()))
		}
		if n > exactInADouble || n < -exactInADouble {
			return "", v.Refuse(fmt.Sprintf(
				"is a number past 2^53 - 1, which is not exact everywhere (%s); a 64-bit integer "+
					"that large is written as a text of its digits", x.String()))
		}
		return x.String(), nil
	default:
		return "", v.wrongKind(fmt.Sprintf("is not a number (%v)", v.value))
	}
}

// ValueSigned64 reads a 64-bit signed value (see wide).
func ValueSigned64(v PayloadValue) (int64, error) {
	text, err := v.wide(true)
	if err != nil {
		return 0, err
	}
	n, err := strconv.ParseInt(text, 10, 64)
	if err != nil {
		return 0, v.Refuse(fmt.Sprintf("does not fit the width its schema declares (%s)", text))
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
		return 0, v.Refuse(fmt.Sprintf("does not fit the width its schema declares (%s)", text))
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
	whole, err := n.Int64()
	if err != nil {
		return 0, v.Refuse(fmt.Sprintf("is not a whole number (%s)", n.String()))
	}
	narrowed := T(whole)
	if int64(narrowed) != whole {
		return 0, v.Refuse(fmt.Sprintf("does not fit the width its schema declares (%s)", n.String()))
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
	if err != nil || whole < 0 {
		return 0, v.Refuse(fmt.Sprintf("is not a whole number at or above zero (%s)", n.String()))
	}
	narrowed := T(whole)
	if int64(narrowed) != whole {
		return 0, v.Refuse(fmt.Sprintf("does not fit the width its schema declares (%s)", n.String()))
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
		return 0, v.Refuse(fmt.Sprintf("is not a number (%s)", n.String()))
	}
	if _, single := any(T(0)).(float32); single && math.Abs(wide) > math.MaxFloat32 {
		return 0, v.Refuse(fmt.Sprintf("does not fit the width its schema declares (%s)", n.String()))
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
func (v PayloadValue) Name() (string, error) {
	s, ok := v.value.(string)
	if !ok {
		return "", v.wrongKind(fmt.Sprintf("is not a text (%v)", v.value))
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
			return nil, v.Refuse("carries a character above U+00FF, which no single byte spells")
		}
		out = append(out, byte(r))
	}
	if len(out) > bound {
		return nil, v.pastBytes(len(out), bound)
	}
	return out, nil
}

func (v PayloadValue) pastBytes(held, bound int) error {
	return v.Refuse(fmt.Sprintf("is %d bytes, past the %d its type declares", held, bound))
}

// ReadList reads a value of `list<T>[bound]`: an array of at most bound
// elements, each read by read at the path of its index, in order.
//
// The bound is held before any element is read: an array longer than bound is
// refused whole, and no element of it is decoded (ADR 0077, section 2).
func ReadList[T any](v PayloadValue, bound int, read func(PayloadValue) (T, error)) ([]T, error) {
	items, ok := v.value.([]any)
	if !ok {
		return nil, v.wrongKind("is not a list")
	}
	if len(items) > bound {
		return nil, v.Refuse(fmt.Sprintf("holds %d elements, past the %d its type declares", len(items), bound))
	}
	out := make([]T, 0, len(items))
	for index, item := range items {
		element, err := read(PayloadValue{value: item, path: joinPath(v.path, strconv.Itoa(index))})
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
	record := PayloadRecord{members: members, prefix: v.path}
	if defaults != "" {
		fallback, err := defaultsOf(defaults)
		if err != nil {
			return PayloadRecord{}, err
		}
		record.defaults = fallback
	}
	return record, nil
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

// PayloadString reads a text field as the text it carries; PayloadValue.Text
// holds a text to the bound its type declares.
func PayloadString(obj PayloadFields, name string) (string, error) {
	v, err := field(obj, name)
	if err != nil {
		return "", err
	}
	return v.Name()
}

// PayloadBytes reads a byte-string field (see PayloadValue.Bytes), held to no
// bound; PayloadValue.Bytes holds it to the one its type declares.
func PayloadBytes(obj PayloadFields, name string) ([]byte, error) {
	v, err := field(obj, name)
	if err != nil {
		return nil, err
	}
	return v.Bytes(math.MaxInt)
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
	if len(items) > bound {
		return fmt.Errorf("'%s' holds %d elements, past the %d its type declares", path, len(items), bound)
	}
	for index, item := range items {
		if err := each(joinPath(path, strconv.Itoa(index)), item); err != nil {
			return err
		}
	}
	return nil
}
