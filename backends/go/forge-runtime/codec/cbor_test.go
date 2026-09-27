// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package codec

// The same four properties the Rust runtime's own tests pin
// (backends/rust/forge-runtime/src/cbor.rs): shortest heads on write, any
// valid head length on read, refusal of what an entry cannot be, and a skip
// that takes an unknown value whole and refuses one too deep. What a
// generated codec does with these is the conformance harness's
// (codec_cbor_map), which this file does not repeat.

import (
	"bytes"
	"errors"
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
	for _, b := range [][]byte{{0xbf}, {0x1c}, {0x61, 'a'}} {
		c := NewSceCursor(b)
		if _, err := CborReadUint(&c); !errors.Is(err, ErrCborMalformed) {
			t.Errorf("% x: %v, want ErrCborMalformed", b, err)
		}
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
