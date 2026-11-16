// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package codec

import (
	"bytes"
	"testing"

	"github.com/newmassrael/sce-forge-runtime/optional"
)

// A Bytes or a Text is made only within its bound, and its zero value is one
// of its type: the fewest units the bound admits.

// twoToFour is a bound of two to four bytes.
type twoToFour struct{}

func (twoToFour) Least() int { return 2 }
func (twoToFour) Most() int  { return 4 }

// none is the bound of an element the protocol bounds by nothing.
type none struct{}

func (none) Least() int { return 0 }
func (none) Most() int  { return Unbounded }

func TestMakeBytesHoldsBytesWithinTheBound(t *testing.T) {
	for _, n := range []int{2, 3, 4} {
		in := bytes.Repeat([]byte{0xAB}, n)
		b, err := MakeBytes[twoToFour](in)
		if err != nil {
			t.Fatalf("%d bytes: refused: %v", n, err)
		}
		if !bytes.Equal(b.Bytes(), in) || b.Len() != n {
			t.Errorf("%d bytes: holds % x (len %d)", n, b.Bytes(), b.Len())
		}
	}
}

func TestMakeBytesRefusesBytesOutsideTheBound(t *testing.T) {
	for _, n := range []int{0, 1, 5} {
		if _, err := MakeBytes[twoToFour](make([]byte, n)); err != ErrLengthOutOfRange {
			t.Errorf("%d bytes: err = %v, want ErrLengthOutOfRange", n, err)
		}
	}
}

func TestTheZeroBytesPresentsTheFewestZeros(t *testing.T) {
	var b Bytes[twoToFour]
	if !bytes.Equal(b.Bytes(), []byte{0, 0}) || b.Len() != 2 {
		t.Errorf("zero value holds % x (len %d), want 00 00", b.Bytes(), b.Len())
	}
	var u Bytes[none]
	if len(u.Bytes()) != 0 || u.Len() != 0 {
		t.Errorf("zero value of no least holds % x", u.Bytes())
	}
}

func TestAnUnboundedBytesHoldsAnyLength(t *testing.T) {
	if _, err := MakeBytes[none](make([]byte, 1<<16)); err != nil {
		t.Errorf("refused: %v", err)
	}
}

func TestMakeTextCountsUTF8Bytes(t *testing.T) {
	// A character of two UTF-8 bytes, spelled by its bytes.
	twoBytes := string([]byte{0xC3, 0xA9})
	// Two such characters: four bytes, within the bound.
	if s, err := MakeText[twoToFour](twoBytes + twoBytes); err != nil || s.String() != twoBytes+twoBytes || s.Len() != 4 {
		t.Errorf("four UTF-8 bytes: %q, %v", s.String(), err)
	}
	// Three such characters: six bytes, past it.
	if _, err := MakeText[twoToFour](twoBytes + twoBytes + twoBytes); err != ErrLengthOutOfRange {
		t.Errorf("six UTF-8 bytes: err = %v, want ErrLengthOutOfRange", err)
	}
	if _, err := MakeText[twoToFour]("a"); err != ErrLengthOutOfRange {
		t.Errorf("one byte: err = %v, want ErrLengthOutOfRange", err)
	}
}

func TestMakeTextRefusesMalformedUTF8(t *testing.T) {
	if _, err := MakeText[twoToFour]("a\xffb"); err != ErrInvalidUTF8 {
		t.Errorf("err = %v, want ErrInvalidUTF8", err)
	}
}

func TestTheZeroTextPresentsTheFewestNULs(t *testing.T) {
	var s Text[twoToFour]
	if s.String() != "\x00\x00" || s.Len() != 2 {
		t.Errorf("zero value holds %q (len %d)", s.String(), s.Len())
	}
}

func TestAnAbsentByteStringOrTextIsMadeAbsent(t *testing.T) {
	b, err := MakeOptionalBytes[twoToFour](optional.None[[]byte]())
	if err != nil || b.IsPresent() {
		t.Errorf("bytes: made %v, err = %v; want absent", b, err)
	}
	s, err := MakeOptionalText[twoToFour](optional.None[string]())
	if err != nil || s.IsPresent() {
		t.Errorf("text: made %v, err = %v; want absent", s, err)
	}
}

func TestAPresentByteStringOrTextIsHeldToItsBound(t *testing.T) {
	b, err := MakeOptionalBytes[twoToFour](optional.Some([]byte{1, 2, 3}))
	if err != nil || !bytes.Equal(b.Proven().Bytes(), []byte{1, 2, 3}) {
		t.Errorf("bytes: made %v, err = %v", b, err)
	}
	if _, err := MakeOptionalBytes[twoToFour](optional.Some([]byte{1})); err != ErrLengthOutOfRange {
		t.Errorf("one byte: err = %v, want ErrLengthOutOfRange", err)
	}
	if _, err := MakeOptionalText[twoToFour](optional.Some("a\xffb")); err != ErrInvalidUTF8 {
		t.Errorf("malformed text: err = %v, want ErrInvalidUTF8", err)
	}
}

func TestAnOptionalBoundedValueIsReadAsWhatItHolds(t *testing.T) {
	held, _ := MakeBytes[twoToFour]([]byte{7, 8})
	if got := OptionalBytes(optional.Some(held)); !bytes.Equal(got.Proven(), []byte{7, 8}) {
		t.Errorf("bytes: read %v", got)
	}
	if got := OptionalBytes(optional.None[Bytes[twoToFour]]()); got.IsPresent() {
		t.Errorf("absent bytes: read %v", got)
	}
	text, _ := MakeText[twoToFour]("ab")
	if got := OptionalText(optional.Some(text)); got.Proven() != "ab" {
		t.Errorf("text: read %v", got)
	}
}
