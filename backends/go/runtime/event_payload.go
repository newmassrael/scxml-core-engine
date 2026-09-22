// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import (
	"encoding/json"
	"fmt"
	"strings"
)

// Lifting an event's typed `_event.data` view out of the data it carries.
//
// NL→IR Item C1 Path A gives a schema'd event a typed payload that the
// natively lowered guards read. One producer fills it: the generated
// `Raise<Event>` inject seam. Every other producer — `<send>` with `<param>`,
// namelist or `<content>`, an invoke forwarding an event either way,
// autoforward, BasicHTTP, mesh — fills `EventMetadata.Data`, the wire W3C
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
	dec := json.NewDecoder(strings.NewReader(trimmed))
	// A whole number that does not fit a float64 must survive the decode, so
	// numbers are kept in their written spelling and converted per field.
	dec.UseNumber()
	var decoded any
	if err := dec.Decode(&decoded); err != nil {
		return nil, fmt.Errorf("the event's data is not JSON: %w", err)
	}
	obj, ok := decoded.(map[string]any)
	if !ok {
		return nil, fmt.Errorf(
			"the event's data is a bare value, and this event's schema declares named fields")
	}
	return PayloadFields(obj), nil
}

// field is the value `name` carries, or a refusal naming what is missing.
func field(obj PayloadFields, name string) (any, error) {
	v, ok := obj[name]
	if !ok {
		return nil, fmt.Errorf("the event's data has no %q", name)
	}
	return v, nil
}

// number is a field's value as the JSON number it was written as.
//
// ⚠ A truth value is not a number here. JSON spells both, and a schema that
// declared uint32 and received `true` has been handed something its own type
// says cannot occur.
func number(obj PayloadFields, name string) (json.Number, error) {
	v, err := field(obj, name)
	if err != nil {
		return "", err
	}
	n, ok := v.(json.Number)
	if !ok {
		return "", fmt.Errorf("%q is not a number (%v)", name, v)
	}
	return n, nil
}

// PayloadSigned reads a signed whole number field at the width its schema
// declared. A value the width cannot hold is refused rather than wrapped —
// silently truncating is how a guard on a boundary answers for a value the
// document never received.
func PayloadSigned[T int8 | int16 | int32 | int64](obj PayloadFields, name string) (T, error) {
	n, err := number(obj, name)
	if err != nil {
		return 0, err
	}
	v, err := n.Int64()
	if err != nil {
		return 0, fmt.Errorf("%q is not a whole number (%s)", name, n.String())
	}
	narrowed := T(v)
	if int64(narrowed) != v {
		return 0, fmt.Errorf("%q does not fit the width its schema declares (%s)", name, n.String())
	}
	return narrowed, nil
}

// PayloadUnsigned reads an unsigned whole number field at its declared width.
func PayloadUnsigned[T uint8 | uint16 | uint32 | uint64](obj PayloadFields, name string) (T, error) {
	n, err := number(obj, name)
	if err != nil {
		return 0, err
	}
	v, err := n.Int64()
	if err != nil || v < 0 {
		return 0, fmt.Errorf("%q is not a whole number at or above zero (%s)", name, n.String())
	}
	narrowed := T(v)
	if int64(narrowed) != v {
		return 0, fmt.Errorf("%q does not fit the width its schema declares (%s)", name, n.String())
	}
	return narrowed, nil
}

// PayloadFloat reads a fractional field at its declared width.
func PayloadFloat[T float32 | float64](obj PayloadFields, name string) (T, error) {
	n, err := number(obj, name)
	if err != nil {
		return 0, err
	}
	v, err := n.Float64()
	if err != nil {
		return 0, fmt.Errorf("%q is not a number (%s)", name, n.String())
	}
	return T(v), nil
}

// PayloadBool reads a truth-value field.
func PayloadBool(obj PayloadFields, name string) (bool, error) {
	v, err := field(obj, name)
	if err != nil {
		return false, err
	}
	b, ok := v.(bool)
	if !ok {
		return false, fmt.Errorf("%q is not a truth value (%v)", name, v)
	}
	return b, nil
}

// PayloadString reads a text field.
func PayloadString(obj PayloadFields, name string) (string, error) {
	v, err := field(obj, name)
	if err != nil {
		return "", err
	}
	s, ok := v.(string)
	if !ok {
		return "", fmt.Errorf("%q is not a text (%v)", name, v)
	}
	return s, nil
}

// PayloadBytes reads a byte-string field.
//
// JSON has no byte string, so the wire carries the byte-exact Latin-1 text the
// inject seam writes, and this reads it back the same way: every one of the
// 256 values is one character and back. Printable ASCII — what a bytes guard
// compares — is the same bytes under either reading.
func PayloadBytes(obj PayloadFields, name string) ([]byte, error) {
	s, err := PayloadString(obj, name)
	if err != nil {
		return nil, err
	}
	out := make([]byte, 0, len(s))
	for _, r := range s {
		if r > 0xFF {
			return nil, fmt.Errorf(
				"%q carries a character above U+00FF, which no single byte spells", name)
		}
		out = append(out, byte(r))
	}
	return out, nil
}

// PayloadJSON is the wire spelling of the fields an inject seam was given, so
// that the script engine binds `_event.data` to the same values the typed
// carrier holds. A byte string is written as its Latin-1 text (see
// PayloadBytes).
func PayloadJSON(fields map[string]any) string {
	encoded, err := json.Marshal(fields)
	if err != nil {
		// Every value here comes from a schema's own field types, all of which
		// marshal; there is no input the caller could give that lands here.
		return ""
	}
	return string(encoded)
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
