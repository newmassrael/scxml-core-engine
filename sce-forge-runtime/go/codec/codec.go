// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// Package codec ships the cursor + typed error contract for code
// generated from sce:kind="codec" SCXML documents. Mirrors the Rust
// reference at sce-forge-runtime/rust/src/codec.rs. RFC §synth-5-B L494-519
// pins a per-language cursor + need-more-bytes contract on decode so
// a truncated input never aborts.
//
// The cursor ships PeekSlice / Advance / Remaining plus the ReadVLE*
// readers. Other streaming readers (e.g. a dedicated ReadU8 / ReadTag)
// are not provided until a consumer needs them.
package codec

import "errors"

// ErrNeedMoreBytes is returned by Decode when the cursor's remaining
// buffer is shorter than the codec's declared minimum frame. Caller
// should resume after appending more bytes.
//
// The variant primitive intentionally does NOT need a typed
// ErrUnknownVariantTag — RFC §synth-5-B requires <sce:default> when arms
// don't exhaust the tag domain (build-time
// codec/variant-arm-unreachable otherwise), so the default arm catches
// every unmatched tag at runtime.
//
var ErrNeedMoreBytes = errors.New("sce/codec: need more bytes")

// ErrVLEWidthOverflow is returned when a vle_u<N> field's continuation
// chain implies a value wider than the declared type. RFC §synth-5-B
// `codec/vle-width-overflow`.
var ErrVLEWidthOverflow = errors.New("sce/codec: vle width overflow")

// ErrInvalidUTF8 is returned by Decode when a sce:type="string" field's
// length-prefixed payload is not well-formed UTF-8 (RFC §synth-5-B).
// Forge-fail-fast contract — zenoh-pico itself aliases the
// bytes without validating, but SCE-side codecs reject malformed text
// early so downstream procedures never see a malformed string. Cpp +
// Kotlin runtimes collapse this to their existing truncation sentinel
// (mirrors the VleWidthOverflow declaration-only convention there).
var ErrInvalidUTF8 = errors.New("sce/codec: invalid utf-8")

// ErrTlvChainOverflow is returned by Decode when a `<sce:tlv-chain
// on-overflow="reject">` field has residual cursor bytes after
// `max_depth` entries have been consumed (RFC §synth-5-B). Truncate
// policy silently drops the residual bytes and never raises this
// sentinel; the on-overflow attribute is parser-mandatory so the codec
// emit always picks one of the two policies. Cpp + Kotlin runtimes
// collapse this to their truncation sentinel (mirrors VleWidthOverflow
// declaration-only convention).
var ErrTlvChainOverflow = errors.New("sce/codec: tlv chain overflow")

// ErrBufferOverflow is returned by Encode when a write would exceed
// the destination sink's remaining capacity (RFC §synth-5-B). Only
// bounded sinks (BoundedSink wrapping a caller-owned `[]byte` +
// fixed cap) can return this; the growable BytesSink (wrapping
// `*[]byte`) is effectively infallible.
var ErrBufferOverflow = errors.New("sce/codec: buffer overflow")

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

// readVLEInner reads a base-128 variable-length encoded unsigned value
// of up to maxBits payload width. LSB-first byte order; leading bytes
// use bit 7 as the continuation flag, while the final byte (at shift
// 7*(VLE_LEN-1)) carries a full 8 data bits with no flag. Canonical
// Zenoh ZInt (RFC §synth-5-B Appendix B): ceil((W-1)/7) bytes max, so a
// u64 caps at 9 bytes, not 10.
func (c *SceCursor) readVLEInner(maxBits uint32) (uint64, error) {
	vleLen := (maxBits - 1 + 6) / 7
	finalShift := 7 * (vleLen - 1)
	var value uint64
	var shift uint32
	for i := uint32(0); i < vleLen; i++ {
		if c.Remaining() < 1 {
			return 0, ErrNeedMoreBytes
		}
		b := c.buf[c.pos]
		c.pos++
		if shift == finalShift {
			// Final byte: 8 data bits, continuation bit reused as data.
			// For a sub-octet tail (u16 / u32) refuse overflow.
			allowed := maxBits - shift
			if allowed < 8 && uint64(b) > (uint64(1)<<allowed)-1 {
				return 0, ErrVLEWidthOverflow
			}
			value |= uint64(b) << shift
			return value, nil
		}
		value |= uint64(b&0x7F) << shift
		if (b & 0x80) == 0 {
			return value, nil
		}
		shift += 7
	}
	return 0, ErrVLEWidthOverflow
}

// ReadVLEU16 reads a vle_u16 field (1-3 wire bytes).
func (c *SceCursor) ReadVLEU16() (uint16, error) {
	v, err := c.readVLEInner(16)
	return uint16(v), err
}

// ReadVLEU32 reads a vle_u32 field (1-5 wire bytes).
func (c *SceCursor) ReadVLEU32() (uint32, error) {
	v, err := c.readVLEInner(32)
	return uint32(v), err
}

// ReadVLEU64 reads a vle_u64 field (1-9 wire bytes). Canonical Zenoh ZInt.
func (c *SceCursor) ReadVLEU64() (uint64, error) {
	return c.readVLEInner(64)
}

// ── Write-side sink ──────────────────────────────────────────────

// SceSink is the write-side cursor for codec emit. Generated `Encode`
// bodies accept it by interface so callers own the destination storage
// (a `*[]byte` for growable sinks, a `[]byte` + capacity for bounded
// sinks). The return contract mirrors the decode-side `error` shape:
// `nil` on success, an `error` (typically `ErrBufferOverflow`) on
// failure. Generated bodies propagate via `if err := ...; err != nil
// { return err }`.
type SceSink interface {
	// WriteBytes appends data to the underlying storage. Bounded
	// sinks return ErrBufferOverflow when the destination has
	// insufficient remaining capacity; growable sinks return nil.
	WriteBytes(data []byte) error

	// Position returns bytes written by this sink instance since
	// construction. Used by codec emit for offset-aware writes
	// (DMA-aligned field padding, length-prefix back-patching).
	Position() int
}

// BytesSink is a growable sink backed by a caller-owned `*[]byte`.
// Writes append to the pointed slice; WriteBytes always returns nil.
// Natural sink behind `EncodeToBytes` facades and coalesced-send
// paths that pack multiple frames into one buffer.
type BytesSink struct {
	dst      *[]byte
	startLen int
}

// NewBytesSink wraps dst for append-only writes. Records the slice's
// current length so Position() returns the bytes written by this
// sink instance (delta) — codec emit positional math stays consistent
// when the destination is shared with a coalesced-send prefix.
func NewBytesSink(dst *[]byte) *BytesSink {
	return &BytesSink{dst: dst, startLen: len(*dst)}
}

// WriteBytes appends data to the destination slice. Infallible —
// always returns nil.
func (s *BytesSink) WriteBytes(data []byte) error {
	if len(data) == 0 {
		return nil
	}
	*s.dst = append(*s.dst, data...)
	return nil
}

// Position returns the bytes written since construction.
func (s *BytesSink) Position() int {
	return len(*s.dst) - s.startLen
}

// BoundedSink is a bounded sink backed by a caller-owned `[]byte` +
// position cursor. Returns ErrBufferOverflow when a write would
// exceed the slice's capacity. Natural sink for fixed-frame /
// DMA-aligned call sites.
type BoundedSink struct {
	buf      []byte
	pos      int
	startPos int
}

// NewBoundedSink wraps buf with write position 0. The caller owns the
// storage; the sink does not extend or free it.
func NewBoundedSink(buf []byte) *BoundedSink {
	return &BoundedSink{buf: buf, pos: 0, startPos: 0}
}

// WriteBytes appends data to the bounded slice; returns ErrBufferOverflow
// when len(data) > Remaining().
func (s *BoundedSink) WriteBytes(data []byte) error {
	n := len(data)
	if n == 0 {
		return nil
	}
	if len(s.buf)-s.pos < n {
		return ErrBufferOverflow
	}
	copy(s.buf[s.pos:s.pos+n], data)
	s.pos += n
	return nil
}

// Position returns the bytes written since construction.
func (s *BoundedSink) Position() int {
	return s.pos - s.startPos
}

// Remaining returns the bytes left in the bounded slice.
func (s *BoundedSink) Remaining() int {
	return len(s.buf) - s.pos
}

// Written returns the slice prefix containing the bytes written by
// this sink (excluding any prefix that was present at construction).
func (s *BoundedSink) Written() []byte {
	return s.buf[s.startPos:s.pos]
}
