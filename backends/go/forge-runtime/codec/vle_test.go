// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package codec

import "testing"

// The wire of a vle_u<N> field is a ZInt64 at every width, and the type
// bounds the value: a narrow field reads every encoding of a value it holds,
// the padded ones included, and refuses a value past its width.

func TestVLEU16ReadsAPaddedEncodingOfAValueItHolds(t *testing.T) {
	cases := []struct {
		wire []byte
		want uint16
	}{
		{[]byte{0x81, 0x80, 0x80, 0x00}, 1},
		{[]byte{0x81, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00}, 1},
		{[]byte{0xFF, 0x7F}, 0x3FFF},
		{[]byte{0xFF, 0xFF, 0x80, 0x00}, 0x3FFF},
		{[]byte{0x80, 0x80, 0x01}, 0x4000},
		{[]byte{0x80, 0x80, 0x81, 0x80, 0x00}, 0x4000},
		{[]byte{0xFF, 0xFF, 0x03}, 0xFFFF},
		{[]byte{0xFF, 0xFF, 0x83, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00}, 0xFFFF},
	}
	for _, c := range cases {
		cursor := NewSceCursor(c.wire)
		got, err := cursor.ReadVLEU16()
		if err != nil || got != c.want {
			t.Errorf("% x: got (%d, %v), want (%d, nil)", c.wire, got, err, c.want)
		}
		if cursor.Remaining() != 0 {
			t.Errorf("% x: %d byte(s) left unread", c.wire, cursor.Remaining())
		}
	}
}

func TestVLEU32AndU64ReadAPaddedEncodingOfAValueTheyHold(t *testing.T) {
	nine := []byte{0x81, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00}
	c32 := NewSceCursor(nine)
	if got, err := c32.ReadVLEU32(); err != nil || got != 1 {
		t.Errorf("u32 padded 1: got (%d, %v)", got, err)
	}
	c64 := NewSceCursor(nine)
	if got, err := c64.ReadVLEU64(); err != nil || got != 1 {
		t.Errorf("u64 padded 1: got (%d, %v)", got, err)
	}
	max32 := NewSceCursor([]byte{0xFF, 0xFF, 0xFF, 0xFF, 0x8F, 0x80, 0x80, 0x80, 0x00})
	if got, err := max32.ReadVLEU32(); err != nil || got != 0xFFFFFFFF {
		t.Errorf("u32 padded max: got (%d, %v)", got, err)
	}
}

func TestVLERefusesTheFirstValuePastItsWidth(t *testing.T) {
	c16 := NewSceCursor([]byte{0x80, 0x80, 0x04})
	if _, err := c16.ReadVLEU16(); err != ErrVLEWidthOverflow {
		t.Errorf("u16 65536: err = %v, want ErrVLEWidthOverflow", err)
	}
	c32 := NewSceCursor([]byte{0x80, 0x80, 0x80, 0x80, 0x10})
	if _, err := c32.ReadVLEU32(); err != ErrVLEWidthOverflow {
		t.Errorf("u32 2^32: err = %v, want ErrVLEWidthOverflow", err)
	}
	// A ninth byte with its high bit set is data, not a flag asking for a tenth.
	high := []byte{0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80}
	h32 := NewSceCursor(high)
	if _, err := h32.ReadVLEU32(); err != ErrVLEWidthOverflow {
		t.Errorf("u32 2^63: err = %v, want ErrVLEWidthOverflow", err)
	}
	h64 := NewSceCursor(high)
	if got, err := h64.ReadVLEU64(); err != nil || got != 1<<63 {
		t.Errorf("u64 2^63: got (%d, %v)", got, err)
	}
	max64 := NewSceCursor([]byte{0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF})
	if got, err := max64.ReadVLEU64(); err != nil || got != 0xFFFFFFFFFFFFFFFF {
		t.Errorf("u64 max: got (%d, %v)", got, err)
	}
}

func TestVLEThatTheBufferEndsInsideNeedsMoreBytes(t *testing.T) {
	short := []byte{0x81, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80}
	c16 := NewSceCursor(short)
	if _, err := c16.ReadVLEU16(); err != ErrNeedMoreBytes {
		t.Errorf("u16: err = %v, want ErrNeedMoreBytes", err)
	}
	c64 := NewSceCursor(short)
	if _, err := c64.ReadVLEU64(); err != ErrNeedMoreBytes {
		t.Errorf("u64: err = %v, want ErrNeedMoreBytes", err)
	}
}
