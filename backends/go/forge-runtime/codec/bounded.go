// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package codec

import (
	"math"
	"strings"
	"unicode/utf8"
)

// The bound a codec's byte string or text is held to, as a type
// (docs/adr/0077 §3, docs/adr/0065 §6 R1).
//
// A codec printed from its plan (docs/adr/0081 §2) presents each byte string
// as a Bytes and each text as a Text of a bound type the codec names, one per
// element. Go has no type that carries a number, so the refinement is an
// abstract type: a struct with an unexported field, made only by MakeBytes or
// MakeText, which check the bound once, where the value is made. A value
// outside the bound is then no value of the type, and the encode over it has
// nothing of the bound to refuse, as the Rust and C++ encodes have nothing
// over their bounded types. Mirrors `Bounded` of the Rust runtime and
// `SCE::Forge::Bytes` and `SCE::Forge::Text` of the C++ runtime.

// Unbounded is the Most of a bound the protocol states no most of
// (docs/adr/0077 §7): the largest int, past which no slice or string has a
// length, so a bound holds it with no second shape, as the C++ runtime's
// `kUnbounded` and the Rust runtime's `usize::MAX` do.
const Unbounded = math.MaxInt

// Bound is the bound of one element: at least Least bytes and at most Most.
// A codec names each element's bound by a zero-size type whose methods return
// the two numbers, so a Bytes or a Text of it carries its bound in its type
// and costs nothing to hold.
type Bound interface {
	// Least is the fewest bytes the element holds.
	Least() int
	// Most is the most bytes the element holds, Unbounded where the protocol
	// states no most.
	Most() int
}

// fewest returns the fewest bytes the bound B admits.
func fewest[B Bound]() int {
	var bound B
	return bound.Least()
}

// within reports whether n bytes lie within the bound B.
func within[B Bound](n int) bool {
	var bound B
	return n >= bound.Least() && n <= bound.Most()
}

// Bytes is a byte string held to its bound B. Its zero value presents the
// fewest bytes B admits, all zero, so a value declared and not made is still
// one of its type; any other value is made by MakeBytes.
//
// The value holds the slice it was made of and does not copy it: the bound is
// on the length, which no write to the bytes through Bytes can change, so the
// value holds its bound however the slice is shared.
type Bytes[B Bound] struct {
	held []byte
}

// MakeBytes returns the Bytes that holds b, or ErrLengthOutOfRange where b is
// shorter than the bound's Least or longer than its Most.
func MakeBytes[B Bound](b []byte) (Bytes[B], error) {
	if !within[B](len(b)) {
		return Bytes[B]{}, ErrLengthOutOfRange
	}
	return Bytes[B]{held: b}, nil
}

// Bytes returns the bytes the value holds: the fewest zeros the bound admits
// for the zero value, nil where that is none.
func (b Bytes[B]) Bytes() []byte {
	if b.held == nil {
		if least := fewest[B](); least > 0 {
			return make([]byte, least)
		}
	}
	return b.held
}

// Len returns how many bytes the value holds.
func (b Bytes[B]) Len() int {
	if b.held == nil {
		return fewest[B]()
	}
	return len(b.held)
}

// Text is a text held to its bound B, counted in the UTF-8 bytes it encodes
// to, as every backend counts a text's length on the wire. A Go string is
// those bytes, and a Text holds only well-formed UTF-8, so the decode of what
// its encode writes reads it back. Its zero value presents the fewest bytes B
// admits, each a NUL, which is well-formed UTF-8; any other value is made by
// MakeText.
type Text[B Bound] struct {
	held string
}

// MakeText returns the Text that holds s, or ErrInvalidUTF8 where s is not
// well-formed UTF-8 and ErrLengthOutOfRange where its bytes are fewer than the
// bound's Least or more than its Most.
func MakeText[B Bound](s string) (Text[B], error) {
	if !utf8.ValidString(s) {
		return Text[B]{}, ErrInvalidUTF8
	}
	if !within[B](len(s)) {
		return Text[B]{}, ErrLengthOutOfRange
	}
	return Text[B]{held: s}, nil
}

// String returns the text the value holds: the fewest NULs the bound admits
// for the zero value.
func (t Text[B]) String() string {
	if t.held == "" {
		if least := fewest[B](); least > 0 {
			return strings.Repeat("\x00", least)
		}
	}
	return t.held
}

// Len returns how many UTF-8 bytes the value holds.
func (t Text[B]) Len() int {
	if t.held == "" {
		return fewest[B]()
	}
	return len(t.held)
}
