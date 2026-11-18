// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package codec

// The properties the Rust runtime's own tests pin
// (backends/rust/forge-runtime/src/cbor.rs): shortest heads and floats on
// write, any valid head length and float precision on read, a signed integer
// at every head boundary, each refusal by its kind (docs/adr/0101 §3.3), a
// union's one-entry map, and a skip that takes an unknown value whole and
// refuses one too deep or of indefinite length. What a generated codec does
// with these is the conformance harness's (codec_cbor_map), which this file
// does not repeat.

import (
	"bytes"
	"errors"
	"math"
	"testing"
)

func cborWritten(t *testing.T, write func(SceSink) error) []byte {
	t.Helper()
	var out []byte
	if err := write(NewBytesSink(&out)); err != nil {
		t.Fatalf("a write into a growable sink refused: %v", err)
	}
	return out
}

func TestCborEveryHeadIsWrittenInItsShortestForm(t *testing.T) {
	// RFC 8949 Appendix A: 0, 23, 24, 255, 256, 65535, 65536, 2^32.
	cases := []struct {
		write func(SceSink) error
		want  []byte
	}{
		{func(w SceSink) error { return CborWriteUint(w, 0) }, []byte{0x00}},
		{func(w SceSink) error { return CborWriteUint(w, 23) }, []byte{0x17}},
		{func(w SceSink) error { return CborWriteUint(w, 24) }, []byte{0x18, 0x18}},
		{func(w SceSink) error { return CborWriteUint(w, 255) }, []byte{0x18, 0xff}},
		{func(w SceSink) error { return CborWriteUint(w, 256) }, []byte{0x19, 0x01, 0x00}},
		{func(w SceSink) error { return CborWriteUint(w, 65535) }, []byte{0x19, 0xff, 0xff}},
		{func(w SceSink) error { return CborWriteUint(w, 65536) }, []byte{0x1a, 0x00, 0x01, 0x00, 0x00}},
		{func(w SceSink) error { return CborWriteUint(w, 1<<32) }, []byte{0x1b, 0, 0, 0, 1, 0, 0, 0, 0}},
		{func(w SceSink) error { return CborWriteText(w, "a") }, []byte{0x61, 'a'}},
		{func(w SceSink) error { return CborWriteBytes(w, []byte{1, 2}) }, []byte{0x42, 1, 2}},
		{func(w SceSink) error { return CborWriteBool(w, true) }, []byte{0xf5}},
		{func(w SceSink) error { return CborWriteMapHead(w, 2) }, []byte{0xa2}},
	}
	for i, c := range cases {
		if got := cborWritten(t, c.write); !bytes.Equal(got, c.want) {
			t.Errorf("case %d: wrote % x, want % x", i, got, c.want)
		}
	}
}

func TestCborAnArrayHeadIsWrittenShortestAndReadAsItsCountAndNoOtherItemIs(t *testing.T) {
	if got := cborWritten(t, func(w SceSink) error { return CborWriteArrayHead(w, 2) }); !bytes.Equal(got, []byte{0x82}) {
		t.Errorf("array head of 2: % x", got)
	}
	if got := cborWritten(t, func(w SceSink) error { return CborWriteArrayHead(w, 24) }); !bytes.Equal(got, []byte{0x98, 0x18}) {
		t.Errorf("array head of 24: % x", got)
	}
	three := NewSceCursor([]byte{0x83})
	if v, err := CborReadArrayLen(&three); err != nil || v != 3 {
		t.Errorf("an array of three: %d, %v", v, err)
	}
	wide := NewSceCursor([]byte{0x98, 0x18})
	if v, err := CborReadArrayLen(&wide); err != nil || v != 24 {
		t.Errorf("an array of 24: %d, %v", v, err)
	}
	// An indefinite-length array, and a map where an array stands.
	indefinite := NewSceCursor([]byte{0x9f})
	if _, err := CborReadArrayLen(&indefinite); !errors.Is(err, ErrCborMalformed) {
		t.Errorf("an indefinite-length array: %v, want ErrCborMalformed", err)
	}
	m := NewSceCursor([]byte{0xa1})
	if _, err := CborReadArrayLen(&m); !errors.Is(err, ErrCborWrongType) {
		t.Errorf("a map read as an array: %v, want ErrCborWrongType", err)
	}
}

func TestCborAHeadIsReadInAnyValidLength(t *testing.T) {
	// 1 written in its 2-byte form still reads as 1.
	c := NewSceCursor([]byte{0x18, 0x01})
	if v, err := CborReadUint(&c); err != nil || v != 1 {
		t.Errorf("1 in a 2-byte head: %d, %v", v, err)
	}
	d := NewSceCursor([]byte{0x1b, 0, 0, 0, 1, 0, 0, 0, 0})
	if v, err := CborReadUint(&d); err != nil || v != 1<<32 {
		t.Errorf("2^32: %d, %v", v, err)
	}
}

func TestCborWhatTheCodecCannotReadIsRefused(t *testing.T) {
	// Indefinite-length map, reserved additional information, another major
	// type, a width the entry cannot hold, a wrong exact length, a text past
	// its bound, a text that is not UTF-8.
	for _, b := range [][]byte{{0xbf}, {0x1c}} {
		c := NewSceCursor(b)
		if _, err := CborReadUint(&c); !errors.Is(err, ErrCborMalformed) {
			t.Errorf("% x: %v, want ErrCborMalformed", b, err)
		}
	}
	a := NewSceCursor([]byte{0x61, 'a'})
	if _, err := CborReadUint(&a); !errors.Is(err, ErrCborWrongType) {
		t.Errorf("a text read as an unsigned integer: %v, want ErrCborWrongType", err)
	}
	m := NewSceCursor([]byte{0xbf})
	if _, err := CborReadMapLen(&m); !errors.Is(err, ErrCborMalformed) {
		t.Errorf("indefinite-length map: %v", err)
	}
	u := NewSceCursor([]byte{0x19, 0x01, 0x00})
	if _, err := CborReadUintUpto(&u, 255); !errors.Is(err, ErrCborOutOfRange) {
		t.Errorf("256 in a uint8: %v", err)
	}
	s := NewSceCursor([]byte{0x42, 1, 2})
	if _, err := CborReadBytesExact(&s, 16); !errors.Is(err, ErrCborWrongLength) {
		t.Errorf("two bytes where sixteen are declared: %v", err)
	}
	x := NewSceCursor([]byte{0x62, 'a', 'b'})
	if _, err := CborReadText(&x, 1); !errors.Is(err, ErrCborOutOfRange) {
		t.Errorf("a text past its bound: %v", err)
	}
	y := NewSceCursor([]byte{0x61, 0xff})
	if _, err := CborReadText(&y, -1); !errors.Is(err, ErrInvalidUTF8) {
		t.Errorf("a text that is not UTF-8: %v", err)
	}
}

func TestCborAnUnknownValueIsSkippedWholeAndADeepOneRefused(t *testing.T) {
	// {1: [2, {3: h'00'}]} then 7: the skip lands on the 7.
	c := NewSceCursor([]byte{0xa1, 0x01, 0x82, 0x02, 0xa1, 0x03, 0x41, 0x00, 0x07})
	if err := CborSkip(&c); err != nil {
		t.Fatalf("a nested value is not skipped: %v", err)
	}
	if v, err := CborReadUint(&c); err != nil || v != 7 {
		t.Errorf("the skip lands on %d, %v; want 7", v, err)
	}
	deep := append(bytes.Repeat([]byte{0x81}, 20), 0x00)
	d := NewSceCursor(deep)
	if err := CborSkip(&d); !errors.Is(err, ErrCborTooDeep) {
		t.Errorf("twenty nested arrays: %v, want ErrCborTooDeep", err)
	}
}

func TestCborASkippedValueOfIndefiniteLengthIsRefusedAtAnyDepth(t *testing.T) {
	for _, b := range [][]byte{
		{0x9f, 0x01, 0xff},
		{0xbf, 0x01, 0x02, 0xff},
		{0x7f, 0x61, 'a', 0xff},
		{0x5f, 0x41, 0x00, 0xff},
		// Nested one and two levels down.
		{0x81, 0x9f, 0xff},
		{0xa1, 0x01, 0x81, 0x7f, 0xff},
	} {
		c := NewSceCursor(b)
		if err := CborSkip(&c); !errors.Is(err, ErrCborMalformed) {
			t.Errorf("skip of % x: %v, want ErrCborMalformed", b, err)
		}
	}
}

// cborSigned is a signed integer at each boundary of a head's length, on
// both sides of zero: the argument of a negative number is -1 - n, so -24 is
// the last of one byte and -25 the first of two.
var cborSigned = []struct {
	value int64
	bytes []byte
}{
	{0, []byte{0x00}},
	{23, []byte{0x17}},
	{24, []byte{0x18, 0x18}},
	{-1, []byte{0x20}},
	{-24, []byte{0x37}},
	{-25, []byte{0x38, 0x18}},
	{-256, []byte{0x38, 0xff}},
	{-257, []byte{0x39, 0x01, 0x00}},
	{-500, []byte{0x39, 0x01, 0xf3}},
	{-65536, []byte{0x39, 0xff, 0xff}},
	{-65537, []byte{0x3a, 0x00, 0x01, 0x00, 0x00}},
	{-(1 << 32), []byte{0x3a, 0xff, 0xff, 0xff, 0xff}},
	{-(1 << 32) - 1, []byte{0x3b, 0, 0, 0, 1, 0, 0, 0, 0}},
	{math.MinInt64, []byte{0x3b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff}},
	{math.MaxInt64, []byte{0x1b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff}},
	{1<<32 - 1, []byte{0x1a, 0xff, 0xff, 0xff, 0xff}},
}

func TestCborASignedIntegerIsWrittenInItsShortestHeadAndReadBack(t *testing.T) {
	for _, s := range cborSigned {
		value := s.value
		if got := cborWritten(t, func(w SceSink) error { return CborWriteInt(w, value) }); !bytes.Equal(got, s.bytes) {
			t.Errorf("%d: wrote % x, want % x", value, got, s.bytes)
		}
		c := NewSceCursor(s.bytes)
		if v, err := CborReadIntWithin(&c, math.MinInt64, math.MaxInt64); err != nil || v != value {
			t.Errorf("% x: read %d, %v; want %d", s.bytes, v, err, value)
		}
	}
}

func TestCborASignedIntegerIsReadInAnyHeadAndHeldToItsRange(t *testing.T) {
	read := func(b []byte, least, greatest int64) (int64, error) {
		c := NewSceCursor(b)
		return CborReadIntWithin(&c, least, greatest)
	}
	// -1 in its two-byte form; -128 and 127, an int8's ends.
	for _, ok := range []struct {
		bytes []byte
		want  int64
	}{
		{[]byte{0x38, 0x00}, -1},
		{[]byte{0x38, 0x7f}, -128},
		{[]byte{0x18, 0x7f}, 127},
	} {
		if v, err := read(ok.bytes, -128, 127); err != nil || v != ok.want {
			t.Errorf("% x as an int8: %d, %v; want %d", ok.bytes, v, err, ok.want)
		}
	}
	// -129 and 128 are past an int8; below int64's least and past its
	// greatest no int64 holds either.
	for _, b := range [][]byte{{0x38, 0x80}, {0x18, 0x80}} {
		if _, err := read(b, -128, 127); !errors.Is(err, ErrCborOutOfRange) {
			t.Errorf("% x as an int8: %v, want ErrCborOutOfRange", b, err)
		}
	}
	for _, b := range [][]byte{{0x3b, 0x80, 0, 0, 0, 0, 0, 0, 0}, {0x1b, 0x80, 0, 0, 0, 0, 0, 0, 0}} {
		if _, err := read(b, math.MinInt64, math.MaxInt64); !errors.Is(err, ErrCborOutOfRange) {
			t.Errorf("% x as an int64: %v, want ErrCborOutOfRange", b, err)
		}
	}
	// A float and a text are no integer.
	for _, b := range [][]byte{{0xf9, 0x3c, 0x00}, {0x61, 'a'}} {
		if _, err := read(b, math.MinInt64, math.MaxInt64); !errors.Is(err, ErrCborWrongType) {
			t.Errorf("% x as an integer: %v, want ErrCborWrongType", b, err)
		}
	}
}

// cborFloats is RFC 8949 Appendix A, and the edges of each precision.
var cborFloats = []struct {
	value float64
	bytes []byte
}{
	{0.0, []byte{0xf9, 0x00, 0x00}},
	{math.Copysign(0, -1), []byte{0xf9, 0x80, 0x00}},
	{1.0, []byte{0xf9, 0x3c, 0x00}},
	{1.1, []byte{0xfb, 0x3f, 0xf1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9a}},
	{1.5, []byte{0xf9, 0x3e, 0x00}},
	// The greatest half, and one past it in a half's spacing there, which a
	// single holds.
	{65504.0, []byte{0xf9, 0x7b, 0xff}},
	{65505.0, []byte{0xfa, 0x47, 0x7f, 0xe1, 0x00}},
	{100000.0, []byte{0xfa, 0x47, 0xc3, 0x50, 0x00}},
	{math.MaxFloat32, []byte{0xfa, 0x7f, 0x7f, 0xff, 0xff}},
	{float64(float32(0.1)), []byte{0xfa, 0x3d, 0xcc, 0xcc, 0xcd}},
	{1.0e300, []byte{0xfb, 0x7e, 0x37, 0xe4, 0x3c, 0x88, 0x00, 0x75, 0x9c}},
	// The least half, a subnormal; the greatest half subnormal; the least
	// half normal; half the least half, which only a single holds.
	{math.Ldexp(1, -24), []byte{0xf9, 0x00, 0x01}},
	{math.Ldexp(1023, -24), []byte{0xf9, 0x03, 0xff}},
	{math.Ldexp(1, -14), []byte{0xf9, 0x04, 0x00}},
	{math.Ldexp(1, -25), []byte{0xfa, 0x33, 0x00, 0x00, 0x00}},
	// One bit below a single's last: only a double holds it.
	{1.0 + math.Ldexp(1, -24), []byte{0xfb, 0x3f, 0xf0, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00}},
	{-4.0, []byte{0xf9, 0xc4, 0x00}},
	{-4.1, []byte{0xfb, 0xc0, 0x10, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66}},
	{math.Inf(1), []byte{0xf9, 0x7c, 0x00}},
	{math.Inf(-1), []byte{0xf9, 0xfc, 0x00}},
}

func TestCborAFloatIsWrittenInTheShortestPrecisionThatHoldsItAndReadBack(t *testing.T) {
	for _, f := range cborFloats {
		value := f.value
		if got := cborWritten(t, func(w SceSink) error { return CborWriteFloat(w, value) }); !bytes.Equal(got, f.bytes) {
			t.Errorf("%g: wrote % x, want % x", value, got, f.bytes)
		}
		c := NewSceCursor(f.bytes)
		if v, err := CborReadFloat(&c); err != nil || math.Float64bits(v) != math.Float64bits(value) {
			t.Errorf("% x: read %g, %v; want %g", f.bytes, v, err, value)
		}
	}
	// Every NaN is the one NaN, whatever its sign and payload.
	for _, nan := range []float64{
		math.NaN(),
		math.Float64frombits(0x7ff0_0000_0000_0001),
		math.Float64frombits(0xfff8_0000_0000_0000),
	} {
		if got := cborWritten(t, func(w SceSink) error { return CborWriteFloat(w, nan) }); !bytes.Equal(got, []byte{0xf9, 0x7e, 0x00}) {
			t.Errorf("a NaN wrote % x, want f9 7e 00", got)
		}
	}
}

func TestCborAFloatIsReadInAnyPrecisionThatHoldsIt(t *testing.T) {
	// 1.5 as a half, a single and a double.
	for _, b := range [][]byte{
		{0xf9, 0x3e, 0x00},
		{0xfa, 0x3f, 0xc0, 0x00, 0x00},
		{0xfb, 0x3f, 0xf8, 0, 0, 0, 0, 0, 0},
	} {
		c := NewSceCursor(b)
		if v, err := CborReadFloat(&c); err != nil || v != 1.5 {
			t.Errorf("% x as a float64: %g, %v", b, v, err)
		}
		d := NewSceCursor(b)
		if v, err := CborReadFloat32(&d); err != nil || v != 1.5 {
			t.Errorf("% x as a float32: %g, %v", b, v, err)
		}
	}
	// A NaN of each precision.
	for _, b := range [][]byte{
		{0xf9, 0x7e, 0x00},
		{0xfa, 0x7f, 0xc0, 0x00, 0x00},
		{0xfb, 0x7f, 0xf8, 0, 0, 0, 0, 0, 0},
	} {
		c := NewSceCursor(b)
		if v, err := CborReadFloat(&c); err != nil || !math.IsNaN(v) {
			t.Errorf("% x as a float64: %g, %v", b, v, err)
		}
		d := NewSceCursor(b)
		if v, err := CborReadFloat32(&d); err != nil || !math.IsNaN(float64(v)) {
			t.Errorf("% x as a float32: %g, %v", b, v, err)
		}
	}
	// 0.1 as a double is no single: a float32 refuses it.
	tenth := []byte{0xfb, 0x3f, 0xb9, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9a}
	c := NewSceCursor(tenth)
	if v, err := CborReadFloat(&c); err != nil || v != 0.1 {
		t.Errorf("0.1 as a float64: %g, %v", v, err)
	}
	d := NewSceCursor(tenth)
	if _, err := CborReadFloat32(&d); !errors.Is(err, ErrCborOutOfRange) {
		t.Errorf("0.1 as a float32: %v, want ErrCborOutOfRange", err)
	}
	// An integer, a truth value, null and a negative integer are no float.
	for _, b := range [][]byte{{0x01}, {0xf5}, {0xf6}, {0x20}} {
		e := NewSceCursor(b)
		if _, err := CborReadFloat(&e); !errors.Is(err, ErrCborWrongType) {
			t.Errorf("% x as a float: %v, want ErrCborWrongType", b, err)
		}
	}
}

func TestCborAnItemThatIsNotWellFormedOrNotInTheProfileIsMalformed(t *testing.T) {
	// An indefinite text and byte string, in chunks, and a break code where
	// an item stands; a simple value in the two-byte form below 32, which
	// is not well-formed (RFC 8949 §3.3); the reserved values 28-30.
	text := NewSceCursor([]byte{0x7f, 0x61, 'a', 0x61, 'b', 0xff})
	if _, err := CborReadText(&text, -1); !errors.Is(err, ErrCborMalformed) {
		t.Errorf("an indefinite text: %v, want ErrCborMalformed", err)
	}
	bs := NewSceCursor([]byte{0x5f, 0x41, 0x00, 0xff})
	if _, err := CborReadBytes(&bs, -1); !errors.Is(err, ErrCborMalformed) {
		t.Errorf("an indefinite byte string: %v, want ErrCborMalformed", err)
	}
	brk := NewSceCursor([]byte{0xff})
	if _, err := CborReadUint(&brk); !errors.Is(err, ErrCborMalformed) {
		t.Errorf("a break code: %v, want ErrCborMalformed", err)
	}
	simple := NewSceCursor([]byte{0xf8, 0x14})
	if _, err := CborReadBool(&simple); !errors.Is(err, ErrCborMalformed) {
		t.Errorf("a two-byte simple value below 32: %v, want ErrCborMalformed", err)
	}
	for _, reserved := range []byte{0x1c, 0x1d, 0x1e} {
		c := NewSceCursor([]byte{reserved})
		if _, _, err := CborReadHead(&c); !errors.Is(err, ErrCborMalformed) {
			t.Errorf("% x: %v, want ErrCborMalformed", reserved, err)
		}
	}
}

func TestCborAnItemThePlaceDoesNotTakeIsOfTheWrongType(t *testing.T) {
	// A negative integer where an unsigned stands, null, a bignum (a tagged
	// item).
	for _, b := range [][]byte{{0x20}, {0xf6}, {0xc2, 0x41, 0x01}} {
		c := NewSceCursor(b)
		if _, err := CborReadUint(&c); !errors.Is(err, ErrCborWrongType) {
			t.Errorf("% x as an unsigned integer: %v, want ErrCborWrongType", b, err)
		}
	}
	// Null, a float whose bits are a truth value's number, an integer.
	for _, b := range [][]byte{{0xf6}, {0xf9, 0x00, 0x14}, {0x01}} {
		c := NewSceCursor(b)
		if _, err := CborReadBool(&c); !errors.Is(err, ErrCborWrongType) {
			t.Errorf("% x as a bool: %v, want ErrCborWrongType", b, err)
		}
	}
	// An unsigned key where a record's text key stands, an array where a map
	// stands.
	k := NewSceCursor([]byte{0x01})
	if _, err := CborReadText(&k, -1); !errors.Is(err, ErrCborWrongType) {
		t.Errorf("an unsigned key as a text: %v, want ErrCborWrongType", err)
	}
	a := NewSceCursor([]byte{0x81, 0x01})
	if _, err := CborReadMapLen(&a); !errors.Is(err, ErrCborWrongType) {
		t.Errorf("an array as a map: %v, want ErrCborWrongType", err)
	}
}

var cborPayment = []string{"card", "cash", "voucher"}

func TestCborAUnionIsAMapOfOneEntryNamedByItsCase(t *testing.T) {
	// docs/adr/0101 §2.8: `cash` of `union:Payment` is a1 64 63 61 73 68 a0.
	cash := []byte{0xa1, 0x64, 'c', 'a', 's', 'h', 0xa0}
	got := cborWritten(t, func(w SceSink) error {
		if err := CborWriteVariant(w, "cash"); err != nil {
			return err
		}
		return CborWriteMapHead(w, 0)
	})
	if !bytes.Equal(got, cash) {
		t.Errorf("cash wrote % x, want % x", got, cash)
	}
	c := NewSceCursor(cash)
	if i, err := CborReadVariant(&c, cborPayment); err != nil || i != 1 {
		t.Errorf("cash read as case %d, %v; want 1", i, err)
	}
	if n, err := CborReadMapLen(&c); err != nil || n != 0 {
		t.Errorf("cash's empty record: %d, %v", n, err)
	}
}

func TestCborAUnionOfNoEntryOrTwoOrOfAnUnknownCaseIsRefused(t *testing.T) {
	cases := []struct {
		bytes []byte
		want  error
	}{
		// No entry, and two: the count is judged before any key is read.
		{[]byte{0xa0}, ErrCborUnionEntries},
		{[]byte{0xa2}, ErrCborUnionEntries},
		// A case no variant names.
		{[]byte{0xa1, 0x64, 'c', 'o', 'i', 'n', 0xa0}, ErrCborUnknownVariant},
		// A key that is no text, and an array where the union's map stands.
		{[]byte{0xa1, 0x01, 0xa0}, ErrCborWrongType},
		{[]byte{0x81, 0xa0}, ErrCborWrongType},
	}
	for _, k := range cases {
		c := NewSceCursor(k.bytes)
		if _, err := CborReadVariant(&c, cborPayment); !errors.Is(err, k.want) {
			t.Errorf("% x as a union: %v, want %v", k.bytes, err, k.want)
		}
	}
}
