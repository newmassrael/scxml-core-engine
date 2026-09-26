// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import (
	"fmt"
	"math"
	"math/big"
	"strconv"
	"unicode/utf8"
)

// A typed host-run request (`sce:request`, SCE Accepted Subset §2.12).
//
// A request crosses to the host as text, like every `<param>`. What makes it
// typed is that each value is held to its field's type where the invocation
// starts — RequestFieldWire — so the text a host reads back is always one its
// field's type parses. The Go port of `sce_rust_runtime::host_processor`'s
// typed-request half; the rule is the same in every runtime.

// RequestFieldType is the type one field of a typed request declares, as the
// generated start site names it.
type RequestFieldType struct {
	kind string
	// Cap is a `bytes` field's `sce:max-size`; zero for every other type.
	Cap int
}

// The scalar request field types. A `bytes` field is RequestFieldBytes(cap).
var (
	RequestFieldUint8   = RequestFieldType{kind: "uint8"}
	RequestFieldUint16  = RequestFieldType{kind: "uint16"}
	RequestFieldUint32  = RequestFieldType{kind: "uint32"}
	RequestFieldUint64  = RequestFieldType{kind: "uint64"}
	RequestFieldInt8    = RequestFieldType{kind: "int8"}
	RequestFieldInt16   = RequestFieldType{kind: "int16"}
	RequestFieldInt32   = RequestFieldType{kind: "int32"}
	RequestFieldInt64   = RequestFieldType{kind: "int64"}
	RequestFieldFloat32 = RequestFieldType{kind: "float32"}
	RequestFieldFloat64 = RequestFieldType{kind: "float64"}
	RequestFieldBool    = RequestFieldType{kind: "bool"}
	RequestFieldString  = RequestFieldType{kind: "string"}
)

// RequestFieldBytes is a `bytes` field of at most capacity bytes.
func RequestFieldBytes(capacity int) RequestFieldType {
	return RequestFieldType{kind: "bytes", Cap: capacity}
}

// wholeBounds is the range an integer field's width holds.
func wholeBounds(kind string) (lo, hi *big.Int, ok bool) {
	bits := map[string]uint{
		"uint8": 8, "uint16": 16, "uint32": 32, "uint64": 64,
		"int8": 8, "int16": 16, "int32": 32, "int64": 64,
	}[kind]
	if bits == 0 {
		return nil, nil, false
	}
	one := big.NewInt(1)
	if kind[0] == 'u' {
		return big.NewInt(0), new(big.Int).Sub(new(big.Int).Lsh(one, bits), one), true
	}
	half := new(big.Int).Lsh(one, bits-1)
	return new(big.Int).Neg(half), new(big.Int).Sub(half, one), true
}

// RequestFieldWire holds one evaluated `<param>` to the field it supplies, and
// spells it for the request.
//
// §scxml-6.4.1: an argument that cannot be evaluated starts nothing, and a
// value the record's field cannot hold is such an argument — the host was
// promised that record. The error is the sentence the error.execution event
// carries, in the words the payload lift uses for a completion that does not
// fit its record, because the two are the same judgement made on the two
// halves of one invocation.
//
// The text returned is the value at the field's type — a whole number's
// digits, a fraction as every untyped `<param>` spells one — which is what the
// Request* readers parse back, so the adapter reading a checked request cannot
// fail. A byte string rides as its byte-exact Latin-1
// text, the spelling a completion's byte field uses.
func RequestFieldWire(value any, name string, ty RequestFieldType) (string, error) {
	if lo, hi, ok := wholeBounds(ty.kind); ok {
		var n *big.Int
		switch v := value.(type) {
		case int:
			n = big.NewInt(int64(v))
		case int64:
			n = big.NewInt(v)
		case float64:
			if math.IsNaN(v) || math.IsInf(v, 0) || v != math.Trunc(v) {
				return "", fmt.Errorf("%q is not a whole number", name)
			}
			n, _ = new(big.Float).SetFloat64(v).Int(nil)
		default:
			return "", fmt.Errorf("%q is not a number", name)
		}
		if n.Cmp(lo) < 0 || n.Cmp(hi) > 0 {
			return "", fmt.Errorf("%q does not fit the width its schema declares (%s)", name, n.String())
		}
		return n.String(), nil
	}
	switch ty.kind {
	case "float32", "float64":
		var f float64
		switch v := value.(type) {
		case int:
			f = float64(v)
		case int64:
			f = float64(v)
		case float64:
			f = v
		default:
			return "", fmt.Errorf("%q is not a number", name)
		}
		// JSON, which a completion's record crosses as, has no spelling for
		// these, so a request may not carry one either.
		if math.IsNaN(f) || math.IsInf(f, 0) {
			return "", fmt.Errorf("%q is not a finite number", name)
		}
		// Spelled as every untyped `<param>` is (ToWireString, ECMAScript's
		// String()), of the value at its declared width — so a host reading
		// the request without the adapter sees what an untyped one would.
		if ty.kind == "float32" {
			narrowed := float32(f)
			if math.IsInf(float64(narrowed), 0) {
				return "", fmt.Errorf("%q does not fit the width its schema declares", name)
			}
			return ToWireString(float64(narrowed)), nil
		}
		return ToWireString(f), nil
	case "bool":
		if b, ok := value.(bool); ok {
			return strconv.FormatBool(b), nil
		}
		return "", fmt.Errorf("%q is not a truth value", name)
	case "string":
		if s, ok := value.(string); ok {
			return s, nil
		}
		return "", fmt.Errorf("%q is not a text", name)
	case "bytes":
		s, ok := value.(string)
		if !ok {
			return "", fmt.Errorf("%q is not a byte string", name)
		}
		for _, r := range s {
			if r > 0xFF {
				return "", fmt.Errorf(
					"%q carries a character above U+00FF, which no single byte spells", name)
			}
		}
		if n := utf8.RuneCountInString(s); n > ty.Cap {
			return "", fmt.Errorf("%q is %d bytes, past the %d its schema declares", name, n, ty.Cap)
		}
		return s, nil
	}
	panic(fmt.Sprintf("RequestFieldWire: unknown field type %q", ty.kind))
}

// requestFieldText is the one text a typed request carries for name.
//
// It panics when there is not exactly one: a request that reached a typed
// adapter was checked field by field where it started, so this is a broken
// promise between two halves of generated code, not a value a host or document
// can supply — and one that would otherwise hand the host a record the
// document never sent.
func requestFieldText(request HostInvokeRequest, name string) string {
	values := request.Params[name]
	if len(values) != 1 {
		panic(fmt.Sprintf(
			"typed request %q does not carry %q exactly once, though its record declares it",
			request.InvokeID, name))
	}
	return values[0]
}

func brokenRequest(request HostInvokeRequest, name, text string) string {
	return fmt.Sprintf(
		"typed request %q carries %q as %q, which its type does not parse, though the start site checked it",
		request.InvokeID, name, text)
}

// RequestSigned reads a signed whole-number field of a checked typed request.
func RequestSigned[T int8 | int16 | int32 | int64](request HostInvokeRequest, name string) T {
	text := requestFieldText(request, name)
	var zero T
	v, err := strconv.ParseInt(text, 10, int(8*sizeOf(zero)))
	if err != nil {
		panic(brokenRequest(request, name, text))
	}
	return T(v)
}

// RequestUnsigned reads an unsigned whole-number field of a checked typed
// request.
func RequestUnsigned[T uint8 | uint16 | uint32 | uint64](request HostInvokeRequest, name string) T {
	text := requestFieldText(request, name)
	var zero T
	v, err := strconv.ParseUint(text, 10, int(8*sizeOf(zero)))
	if err != nil {
		panic(brokenRequest(request, name, text))
	}
	return T(v)
}

// RequestFloat reads a fractional field of a checked typed request.
func RequestFloat[T float32 | float64](request HostInvokeRequest, name string) T {
	text := requestFieldText(request, name)
	var zero T
	v, err := strconv.ParseFloat(text, int(8*sizeOf(zero)))
	if err != nil {
		panic(brokenRequest(request, name, text))
	}
	return T(v)
}

// RequestBool reads a truth-value field of a checked typed request.
func RequestBool(request HostInvokeRequest, name string) bool {
	text := requestFieldText(request, name)
	v, err := strconv.ParseBool(text)
	if err != nil || (text != "true" && text != "false") {
		panic(brokenRequest(request, name, text))
	}
	return v
}

// RequestString reads a text field of a checked typed request.
func RequestString(request HostInvokeRequest, name string) string {
	return requestFieldText(request, name)
}

// RequestBytes reads a byte-string field of a checked typed request, from the
// Latin-1 text RequestFieldWire spelled it as.
func RequestBytes(request HostInvokeRequest, name string) []byte {
	text := requestFieldText(request, name)
	out := make([]byte, 0, len(text))
	for _, r := range text {
		if r > 0xFF {
			panic(brokenRequest(request, name, text))
		}
		out = append(out, byte(r))
	}
	return out
}

// sizeOf is the width in bytes of a fixed-size numeric type argument.
func sizeOf[T int8 | int16 | int32 | int64 | uint8 | uint16 | uint32 | uint64 | float32 | float64](v T) uintptr {
	switch any(v).(type) {
	case int8, uint8:
		return 1
	case int16, uint16:
		return 2
	case int32, uint32, float32:
		return 4
	default:
		return 8
	}
}
