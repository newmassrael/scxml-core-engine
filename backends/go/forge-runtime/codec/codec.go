// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// Package codec ships the cursor + typed error contract for code
// generated from sce:kind="codec" SCXML documents. Mirrors the Rust
// reference at backends/rust/forge-runtime/src/codec.rs. RFC §synth-5-B L494-519
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

// ErrBufferOverflow is returned by Encode when a write would exceed
// the destination sink's remaining capacity (RFC §synth-5-B). Only
// bounded sinks (BoundedSink wrapping a caller-owned `[]byte` +
// fixed cap) can return this; the growable BytesSink (wrapping
// `*[]byte`) is effectively infallible.
var ErrBufferOverflow = errors.New("sce/codec: buffer overflow")

// ErrPresentIfMismatch is returned by Encode when a field gated by a
// `sce:present-if` the codec cannot derive (a chain-membership test, a flag
// input the caller gives, a disjunction, a flag wider than one bit) was given
// while the predicate says it does not belong on the wire, or left out while
// it does. The predicate and the field contradict each other; Encode refuses
// before writing the first byte, so the sink is untouched.
var ErrPresentIfMismatch = errors.New("sce/codec: present-if mismatch")

// ErrUndeclaredEnumValue is returned by Decode when an enum-typed field
// carried a value its declared set does not hold. sce:strict-variants=
// "true" (the default) closes the set, so a carrier value outside it is
// not a value of that type and the frame does not decode. An open set
// carries every value of its carrier and never returns this. Cpp +
// Kotlin runtimes collapse this to their truncation sentinel (mirrors
// the ErrInvalidUTF8 convention).
var ErrUndeclaredEnumValue = errors.New("sce/codec: undeclared enum value")

// ErrLengthOutOfRange is returned when a length is outside the bound it must
// respect. Decode returns it for a byte string's or a text's length outside
// its field's bound, the declared sce:max-size intersected with what its
// length field can say after its sce:length-arith (docs/adr/0077 §1), judged
// before the frame's own length so such a frame is refused and never waited
// on. The typed read of a generated broadcast log returns it for an entry
// whose stored length exceeds the element's largest encoded size: such a
// length is not one the log wrote. Either way it is refused before any slice
// is cut by it. Mirrors the Rust runtime's `CodecError::LengthOutOfRange`.
var ErrLengthOutOfRange = errors.New("sce/codec: length out of range")

// ErrTooManyElements is returned by Decode for a list whose count is past its
// declared max-count, or whose entries, read to the end of the frame, pass
// it; the entry past the bound is refused before it is read
// (docs/adr/0077 §2). Mirrors the Rust runtime's
// `CodecError::TooManyElements`.
var ErrTooManyElements = errors.New("sce/codec: too many elements")

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

// EntriesBacked returns the most entries a list that claims count can hold
// in what is left of the frame, for its decode to reserve. An entry takes a
// byte at least, so what a decode reserves answers to the frame's own length
// and never to a number the frame only states (docs/adr/0077 §7). A list
// that is really longer grows as its entries are read.
func (c *SceCursor) EntriesBacked(count uint64) int {
	left := c.Remaining()
	if count < uint64(left) {
		return int(count)
	}
	return left
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

// zintFlaggedBytes is the bytes of a ZInt that carry a continuation flag:
// a chain of this many flagged bytes ends with a ninth that holds a full 8
// data bits, so no ZInt is longer than nine bytes, whatever the width of
// the field that reads it (Appendix B of the protocol-synthesis RFC).
const zintFlaggedBytes = 8

// readZInt reads one ZInt, the canonical Zenoh variable-length integer, as
// zenoh and zenoh-pico read every one of them: an unsigned 64-bit value,
// LSB-first, in at most nine bytes. Each of the first eight carries 7 data
// bits in its low 7 and a continuation flag in bit 7; a ninth carries a
// full 8 data bits and has no flag, so bit 63 rides in its high bit and no
// chain is longer. A chain the buffer ends inside is ErrNeedMoreBytes.
func (c *SceCursor) readZInt() (uint64, error) {
	var value uint64
	for group := 0; group < zintFlaggedBytes; group++ {
		if c.Remaining() < 1 {
			return 0, ErrNeedMoreBytes
		}
		b := c.buf[c.pos]
		c.pos++
		value |= uint64(b&0x7F) << (7 * group)
		if (b & 0x80) == 0 {
			return value, nil
		}
	}
	if c.Remaining() < 1 {
		return 0, ErrNeedMoreBytes
	}
	last := c.buf[c.pos]
	c.pos++
	return value | uint64(last)<<(7*zintFlaggedBytes), nil
}

// readVLEInner reads a vle_u<N> field of maxBits width: one ZInt
// (readZInt, RFC §synth-5-B Appendix B), whatever the width, and then the
// value held to the width.
// The wire of every width is the same ZInt64, and the type bounds the
// value and not the encoding: a narrow field accepts every encoding of a
// value it holds, the padded ones included, as upstream Zenoh does, and
// refuses a value past 2^maxBits - 1 with ErrVLEWidthOverflow, never a
// longer encoding.
func (c *SceCursor) readVLEInner(maxBits uint32) (uint64, error) {
	value, err := c.readZInt()
	if err != nil {
		return 0, err
	}
	if maxBits < 64 && value>>maxBits != 0 {
		return 0, ErrVLEWidthOverflow
	}
	return value, nil
}

// ReadVLEU16 reads a vle_u16 field: a ZInt of 1-9 wire bytes whose value
// is at most math.MaxUint16.
func (c *SceCursor) ReadVLEU16() (uint16, error) {
	v, err := c.readVLEInner(16)
	return uint16(v), err
}

// ReadVLEU32 reads a vle_u32 field: a ZInt of 1-9 wire bytes whose value
// is at most math.MaxUint32.
func (c *SceCursor) ReadVLEU32() (uint32, error) {
	v, err := c.readVLEInner(32)
	return uint32(v), err
}

// ReadVLEU64 reads a vle_u64 field: a ZInt of 1-9 wire bytes.
func (c *SceCursor) ReadVLEU64() (uint64, error) {
	return c.readVLEInner(64)
}

// writeVLEInner appends value to w as a base-128 VLE of maxBits payload
// width. The write-side counterpart of (*SceCursor).readVLEInner:
// leading bytes carry 7 data bits + a continuation flag; the final byte
// (after at most VLE_LEN-1 continuation bytes) carries a full 8 data
// bits with no flag, so a u64 caps at 9 bytes — canonical Zenoh ZInt
// (RFC §synth-5-B Appendix B). VLE_LEN = ceil((maxBits-1)/7). The sink
// is an interface (no concrete cursor to hang a method on), so the VLE
// writers are package functions over it, mirroring the ReadVLE* methods.
func writeVLEInner(w SceSink, value uint64, maxBits uint32) error {
	contMax := (maxBits-1+6)/7 - 1
	v := value
	var n uint32
	for v >= 0x80 && n < contMax {
		if err := w.WriteBytes([]byte{byte(v&0x7F) | 0x80}); err != nil {
			return err
		}
		v >>= 7
		n++
	}
	return w.WriteBytes([]byte{byte(v)})
}

// WriteVLEU16 appends a vle_u16 field (1-3 wire bytes).
func WriteVLEU16(w SceSink, v uint16) error { return writeVLEInner(w, uint64(v), 16) }

// WriteVLEU32 appends a vle_u32 field (1-5 wire bytes).
func WriteVLEU32(w SceSink, v uint32) error { return writeVLEInner(w, uint64(v), 32) }

// WriteVLEU64 appends a vle_u64 field (1-9 wire bytes). Canonical Zenoh ZInt.
func WriteVLEU64(w SceSink, v uint64) error { return writeVLEInner(w, v, 64) }

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
