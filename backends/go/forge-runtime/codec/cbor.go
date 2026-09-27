// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package codec

// The CBOR (RFC 8949) items a `sce:encoding="cbor"` codec reads and writes
// (SCE_FORGE.md §4.6.1): one definite-length map whose keys are small
// unsigned integers and whose values are unsigned integers, booleans, text
// strings and byte strings.
//
// Mirrors backends/rust/forge-runtime/src/cbor.rs, rule for rule. A
// generated codec calls these; it spells no CBOR of its own.
//
// Writing is deterministic (RFC 8949 §4.2.1): every head in its shortest
// form, so two backends given the same value write the same bytes. Reading
// takes a head in any valid length — what it reads is the value — and
// refuses what the codec cannot read as its entry: a reserved
// additional-information value, an indefinite length, another major type.

import (
	"encoding/binary"
	"errors"
	"unicode/utf8"
)

// ErrCborMalformed: the input is not a map this codec reads — not a
// definite-length map, a key given twice, or an entry of the wrong major
// type.
var ErrCborMalformed = errors.New("sce/codec: cbor malformed")

// ErrCborRequiredKeyMissing: the map lacks an entry declared
// `sce:required="true"`.
var ErrCborRequiredKeyMissing = errors.New("sce/codec: cbor required key missing")

// ErrCborWrongLength: a byte string is not its entry's exact `sce:length`.
var ErrCborWrongLength = errors.New("sce/codec: cbor wrong length")

// ErrCborTooDeep: an unknown entry's value nests deeper than
// CborMaxSkipDepth.
var ErrCborTooDeep = errors.New("sce/codec: cbor too deep")

// ErrCborOutOfRange: a value does not fit its entry's type or
// `sce:max-size`.
var ErrCborOutOfRange = errors.New("sce/codec: cbor out of range")

const (
	// CborMajorUnsigned is major type 0, an unsigned integer.
	CborMajorUnsigned uint8 = 0
	// CborMajorBytes is major type 2, a byte string.
	CborMajorBytes uint8 = 2
	// CborMajorText is major type 3, a UTF-8 text string.
	CborMajorText uint8 = 3
	// CborMajorMap is major type 5, a map.
	CborMajorMap uint8 = 5
	// CborMajorSimple is major type 7, simple values (false = 20, true = 21).
	CborMajorSimple uint8 = 7
	// CborMaxSkipDepth is the deepest an unknown entry's value may nest
	// before a skip refuses it (SCE_FORGE.md §4.6.1).
	CborMaxSkipDepth uint32 = 16
)

// ── Writing ─────────────────────────────────────────────────────────────

// CborWriteHead writes a head of `major` carrying `value`, in its shortest
// form.
func CborWriteHead(w SceSink, major uint8, value uint64) error {
	m := major << 5
	switch {
	case value < 24:
		return w.WriteBytes([]byte{m | uint8(value)})
	case value <= 0xFF:
		return w.WriteBytes([]byte{m | 24, uint8(value)})
	case value <= 0xFFFF:
		b := []byte{m | 25, 0, 0}
		binary.BigEndian.PutUint16(b[1:], uint16(value))
		return w.WriteBytes(b)
	case value <= 0xFFFFFFFF:
		b := []byte{m | 26, 0, 0, 0, 0}
		binary.BigEndian.PutUint32(b[1:], uint32(value))
		return w.WriteBytes(b)
	default:
		b := []byte{m | 27, 0, 0, 0, 0, 0, 0, 0, 0}
		binary.BigEndian.PutUint64(b[1:], value)
		return w.WriteBytes(b)
	}
}

// CborWriteUint writes an unsigned integer.
func CborWriteUint(w SceSink, value uint64) error {
	return CborWriteHead(w, CborMajorUnsigned, value)
}

// CborWriteBool writes false or true.
func CborWriteBool(w SceSink, value bool) error {
	simple := uint8(20)
	if value {
		simple = 21
	}
	return w.WriteBytes([]byte{CborMajorSimple<<5 | simple})
}

// CborWriteText writes a text string.
func CborWriteText(w SceSink, value string) error {
	if err := CborWriteHead(w, CborMajorText, uint64(len(value))); err != nil {
		return err
	}
	return w.WriteBytes([]byte(value))
}

// CborWriteBytes writes a byte string.
func CborWriteBytes(w SceSink, value []byte) error {
	if err := CborWriteHead(w, CborMajorBytes, uint64(len(value))); err != nil {
		return err
	}
	return w.WriteBytes(value)
}

// CborWriteMapHead writes the head of a definite-length map of `entries`
// entries.
func CborWriteMapHead(w SceSink, entries uint64) error {
	return CborWriteHead(w, CborMajorMap, entries)
}

// ── Reading ─────────────────────────────────────────────────────────────

func cborReadBE(c *SceCursor, n int) (uint64, error) {
	b, err := c.PeekSlice(n)
	if err != nil {
		return 0, err
	}
	var v uint64
	for _, x := range b {
		v = v<<8 | uint64(x)
	}
	return v, c.Advance(n)
}

// CborReadHead reads one head: its major type and the value its additional
// information carries (for a string or a map, the length). An indefinite
// length and the reserved values 28–30 are refused.
func CborReadHead(c *SceCursor) (uint8, uint64, error) {
	initial, err := cborReadBE(c, 1)
	if err != nil {
		return 0, 0, err
	}
	major := uint8(initial >> 5)
	info := uint8(initial & 0x1F)
	var value uint64
	switch {
	case info < 24:
		value = uint64(info)
	case info == 24:
		value, err = cborReadBE(c, 1)
	case info == 25:
		value, err = cborReadBE(c, 2)
	case info == 26:
		value, err = cborReadBE(c, 4)
	case info == 27:
		value, err = cborReadBE(c, 8)
	default:
		return 0, 0, ErrCborMalformed
	}
	if err != nil {
		return 0, 0, err
	}
	return major, value, nil
}

func cborExpect(c *SceCursor, major uint8) (uint64, error) {
	m, value, err := CborReadHead(c)
	if err != nil {
		return 0, err
	}
	if m != major {
		return 0, ErrCborMalformed
	}
	return value, nil
}

// CborReadUint reads an unsigned integer.
func CborReadUint(c *SceCursor) (uint64, error) {
	return cborExpect(c, CborMajorUnsigned)
}

// CborReadUintUpto reads an unsigned integer an entry of `max` holds.
func CborReadUintUpto(c *SceCursor, max uint64) (uint64, error) {
	v, err := CborReadUint(c)
	if err != nil {
		return 0, err
	}
	if v > max {
		return 0, ErrCborOutOfRange
	}
	return v, nil
}

// CborReadBool reads false or true.
func CborReadBool(c *SceCursor) (bool, error) {
	m, value, err := CborReadHead(c)
	if err != nil {
		return false, err
	}
	if m != CborMajorSimple || (value != 20 && value != 21) {
		return false, ErrCborMalformed
	}
	return value == 21, nil
}

func cborReadPayload(c *SceCursor, n uint64) ([]byte, error) {
	if n > uint64(c.Remaining()) {
		return nil, ErrNeedMoreBytes
	}
	b, err := c.PeekSlice(int(n))
	if err != nil {
		return nil, err
	}
	out := append([]byte(nil), b...)
	return out, c.Advance(int(n))
}

// CborReadText reads a UTF-8 text string of at most `maxSize` bytes (a
// negative `maxSize`: no bound).
func CborReadText(c *SceCursor, maxSize int64) (string, error) {
	n, err := cborExpect(c, CborMajorText)
	if err != nil {
		return "", err
	}
	if maxSize >= 0 && n > uint64(maxSize) {
		return "", ErrCborOutOfRange
	}
	b, err := cborReadPayload(c, n)
	if err != nil {
		return "", err
	}
	if !utf8.Valid(b) {
		return "", ErrInvalidUTF8
	}
	return string(b), nil
}

// CborReadBytes reads a byte string of at most `maxSize` bytes (a negative
// `maxSize`: no bound).
func CborReadBytes(c *SceCursor, maxSize int64) ([]byte, error) {
	n, err := cborExpect(c, CborMajorBytes)
	if err != nil {
		return nil, err
	}
	if maxSize >= 0 && n > uint64(maxSize) {
		return nil, ErrCborOutOfRange
	}
	return cborReadPayload(c, n)
}

// CborReadBytesExact reads a byte string of exactly `length` bytes.
func CborReadBytesExact(c *SceCursor, length uint64) ([]byte, error) {
	n, err := cborExpect(c, CborMajorBytes)
	if err != nil {
		return nil, err
	}
	if n != length {
		return nil, ErrCborWrongLength
	}
	return cborReadPayload(c, n)
}

// CborReadMapLen reads the head of a definite-length map: how many entries
// follow.
func CborReadMapLen(c *SceCursor) (uint64, error) {
	return cborExpect(c, CborMajorMap)
}

// CborSkip skips one item — the value of a key the codec does not declare —
// and everything nested in it, refusing one nested deeper than
// CborMaxSkipDepth.
func CborSkip(c *SceCursor) error {
	return cborSkipAt(c, 0)
}

func cborSkipAt(c *SceCursor, depth uint32) error {
	if depth >= CborMaxSkipDepth {
		return ErrCborTooDeep
	}
	major, value, err := CborReadHead(c)
	if err != nil {
		return err
	}
	switch major {
	// Unsigned, negative, simple / float: the head is the whole item.
	case 0, 1, 7:
		return nil
	case 2, 3:
		_, err := cborReadPayload(c, value)
		return err
	case 4:
		for i := uint64(0); i < value; i++ {
			if err := cborSkipAt(c, depth+1); err != nil {
				return err
			}
		}
		return nil
	case 5:
		for i := uint64(0); i < value; i++ {
			if err := cborSkipAt(c, depth+1); err != nil {
				return err
			}
			if err := cborSkipAt(c, depth+1); err != nil {
				return err
			}
		}
		return nil
	// A tag: its one enclosed item follows.
	case 6:
		return cborSkipAt(c, depth+1)
	default:
		return ErrCborMalformed
	}
}
