// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// Package codec ships the cursor + typed error contract for code
// generated from sce:kind="codec" SCXML documents. Mirrors the Rust
// reference at sce-forge-runtime/rust/src/codec.rs. RFC §5.B L494-519
// pins a per-language cursor + need-more-bytes contract on decode so
// a truncated input never aborts.
//
// B1-prep ships PeekSlice / Advance / Remaining. Streaming readers
// (ReadU8, ReadVLE*, ReadTag) land in B1-α/β with their first consumer.
package codec

import "errors"

// ErrNeedMoreBytes is returned by Decode when the cursor's remaining
// buffer is shorter than the codec's declared minimum frame. Caller
// should resume after appending more bytes.
//
// B1-α adds ErrVLEWidthOverflow, B1-β adds ErrUnknownVariantTag.
var ErrNeedMoreBytes = errors.New("sce/codec: need more bytes")

// SceCursor is a read-only cursor over a borrowed input slice. Decode
// bodies use PeekSlice to bounds-check + read fixed-offset bytes
// positionally, then Advance after the construction succeeds.
type SceCursor struct {
	buf []byte
	pos int
}

// NewSceCursor wraps buf with cursor position 0.
func NewSceCursor(buf []byte) SceCursor {
	return SceCursor{buf: buf, pos: 0}
}

// Remaining returns the number of bytes from the current position to
// the end of the input.
func (c *SceCursor) Remaining() int {
	return len(c.buf) - c.pos
}

// PeekSlice borrows the next n bytes without advancing the cursor.
// Returns ErrNeedMoreBytes when the cursor's tail is shorter than n.
func (c *SceCursor) PeekSlice(n int) ([]byte, error) {
	if c.Remaining() < n {
		return nil, ErrNeedMoreBytes
	}
	return c.buf[c.pos : c.pos+n], nil
}

// Advance moves the cursor n bytes forward. Returns ErrNeedMoreBytes if
// n would overrun the buffer.
func (c *SceCursor) Advance(n int) error {
	if c.Remaining() < n {
		return ErrNeedMoreBytes
	}
	c.pos += n
	return nil
}
