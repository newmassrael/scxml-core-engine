// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package codec

// The CBOR (RFC 8949) items a `sce:encoding="cbor"` codec reads and writes
// (SCE_FORGE.md §4.6.1, docs/adr/0101): one definite-length map whose keys
// are small unsigned integers and whose values are values of the language,
// each in its one form: an integer, a float, a bool, a text or byte string,
// an array, a map keyed by text, and a union's map of one entry.
//
// Mirrors backends/rust/forge-runtime/src/cbor.rs, rule for rule. A
// generated codec calls these; it spells no CBOR of its own.
//
// Writing is deterministic (RFC 8949 §4.2.1): every head in its shortest
// form and every float in the shortest precision that holds it exactly, so
// two backends given the same value write the same bytes. Reading takes the
// profile of definite lengths (docs/adr/0101 §3.1), and within it every
// well-formed encoding of a value: a head in any valid length, a float in
// any precision that holds the value. It refuses each input by the class
// RFC 8949 §1.2 puts it in (docs/adr/0101 §3.3): an item that is not
// well-formed or has an indefinite length is ErrCborMalformed, an item of a
// major type, simple value or tag the place does not take is
// ErrCborWrongType, and a value outside its type is ErrCborOutOfRange.

import (
	"encoding/binary"
	"errors"
	"math"
	"unicode/utf8"
)

// ErrCborMalformed: an item that is not well-formed CBOR (RFC 8949 Appendix
// F: reserved additional information, a break code where an item stands, a
// two-byte simple value below 32), or one of indefinite length, which the
// codec's profile of definite lengths does not hold (docs/adr/0101 §3.1).
var ErrCborMalformed = errors.New("sce/codec: cbor malformed")

// ErrCborWrongType: an item of a major type, a simple value or a tag the
// type at that place does not take: a text where a number stands, a
// negative integer where an unsigned type stands, null where no list's
// optional element stands, a tagged item, a top-level key that is not an
// unsigned integer, a record's or a union's key that is not a text.
var ErrCborWrongType = errors.New("sce/codec: cbor wrong type")

// ErrCborDuplicateKey: a map holds one of its declared keys twice (RFC 8949
// §5.6).
var ErrCborDuplicateKey = errors.New("sce/codec: cbor duplicate key")

// ErrCborUnknownVariant: a union's one key names none of its cases.
var ErrCborUnknownVariant = errors.New("sce/codec: cbor unknown variant")

// ErrCborUnionEntries: a union's map holds no entry, or more than one.
var ErrCborUnionEntries = errors.New("sce/codec: cbor union entries")

// ErrCborRequiredKeyMissing: the map lacks an entry of a bare type, one that
// is not `optional<T>`.
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
	// CborMajorNegative is major type 1, the negative integer -1 - n for the
	// argument n.
	CborMajorNegative uint8 = 1
	// CborMajorBytes is major type 2, a byte string.
	CborMajorBytes uint8 = 2
	// CborMajorText is major type 3, a UTF-8 text string.
	CborMajorText uint8 = 3
	// CborMajorArray is major type 4, an array.
	CborMajorArray uint8 = 4
	// CborMajorMap is major type 5, a map.
	CborMajorMap uint8 = 5
	// CborMajorSimple is major type 7, simple values (false = 20, true = 21)
	// and floats.
	CborMajorSimple uint8 = 7
	// CborMaxSkipDepth is the deepest an unknown entry's value may nest
	// before a skip refuses it (SCE_FORGE.md §4.6.1).
	CborMaxSkipDepth uint32 = 16
)

// The additional information of a half, a single and a double precision
// float under major type 7 (RFC 8949 §3.3).
const (
	cborInfoHalf   uint8 = 25
	cborInfoSingle uint8 = 26
	cborInfoDouble uint8 = 27
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

// CborWriteInt writes a signed integer: major type 0 for zero or more, major
// type 1 with the argument -1 - n below zero, each with its shortest head
// (RFC 8949 §3.1 and §4.2.1).
func CborWriteInt(w SceSink, value int64) error {
	if value < 0 {
		// -1 - value is at most math.MaxInt64: it neither overflows nor
		// loses a bit as a uint64.
		return CborWriteHead(w, CborMajorNegative, uint64(-1-value))
	}
	return CborWriteHead(w, CborMajorUnsigned, uint64(value))
}

// cborHalfOf answers the half precision float that is x exactly, as its
// bits, and whether one is (IEEE 754 binary16: 5 bits of exponent, 10 of
// fraction, subnormal below 2^-14). A NaN has none.
func cborHalfOf(x float64) (uint16, bool) {
	bits := math.Float64bits(x)
	sign := uint16((bits >> 48) & 0x8000)
	exponent := int((bits >> 52) & 0x7ff)
	fraction := bits & (1<<52 - 1)
	if exponent == 0x7ff {
		// An infinity; a NaN is no half's.
		return sign | 0x7c00, fraction == 0
	}
	if exponent == 0 {
		// Zero, or a double's subnormal, which is far below a half's least.
		return sign, fraction == 0
	}
	power := exponent - 1023
	if power >= -14 && power <= 15 {
		// A half's normal number keeps the top 10 bits of the fraction and
		// nothing below them.
		if fraction&(1<<42-1) != 0 {
			return 0, false
		}
		return sign | uint16(power+15)<<10 | uint16(fraction>>42), true
	}
	if power >= -24 && power <= -15 {
		// A half's subnormal number is k * 2^-24 for k from 1 to 1023.
		significand := fraction | 1<<52
		shift := uint(28 - power)
		if significand&(uint64(1)<<shift-1) != 0 {
			return 0, false
		}
		return sign | uint16(significand>>shift), true
	}
	return 0, false
}

// cborSingleOf answers the single precision float that is x exactly, and
// whether one is. A NaN has none.
func cborSingleOf(x float64) (float32, bool) {
	single := float32(x)
	return single, float64(single) == x
}

// CborWriteFloat writes a float in the shortest of half, single and double
// precision that holds it exactly, and a NaN as f9 7e 00 (RFC 8949 §4.1 and
// §4.2.1, docs/adr/0086 section 9.2). A float32 is written by its value as a
// float64, which a single always holds.
func CborWriteFloat(w SceSink, x float64) error {
	if math.IsNaN(x) {
		return w.WriteBytes([]byte{CborMajorSimple<<5 | cborInfoHalf, 0x7e, 0x00})
	}
	if half, ok := cborHalfOf(x); ok {
		b := []byte{CborMajorSimple<<5 | cborInfoHalf, 0, 0}
		binary.BigEndian.PutUint16(b[1:], half)
		return w.WriteBytes(b)
	}
	if single, ok := cborSingleOf(x); ok {
		b := []byte{CborMajorSimple<<5 | cborInfoSingle, 0, 0, 0, 0}
		binary.BigEndian.PutUint32(b[1:], math.Float32bits(single))
		return w.WriteBytes(b)
	}
	b := []byte{CborMajorSimple<<5 | cborInfoDouble, 0, 0, 0, 0, 0, 0, 0, 0}
	binary.BigEndian.PutUint64(b[1:], math.Float64bits(x))
	return w.WriteBytes(b)
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

// CborWriteArrayHead writes the head of a definite-length array of `items`
// items: a list of records an entry holds.
func CborWriteArrayHead(w SceSink, items uint64) error {
	return CborWriteHead(w, CborMajorArray, items)
}

// CborWriteMapHead writes the head of a definite-length map of `entries`
// entries.
func CborWriteMapHead(w SceSink, entries uint64) error {
	return CborWriteHead(w, CborMajorMap, entries)
}

// CborWriteVariant writes the head of a union's value: a map of one entry
// keyed by the case's variant name as a text string (docs/adr/0101 §2.1).
// The case's record, or the empty map for a case that holds none, is
// written after it.
func CborWriteVariant(w SceSink, name string) error {
	if err := CborWriteMapHead(w, 1); err != nil {
		return err
	}
	return CborWriteText(w, name)
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

// cborHead is one head as the input wrote it: its major type, its additional
// information, which tells a float's precision apart, and its argument.
type cborHead struct {
	major    uint8
	info     uint8
	argument uint64
}

// cborReadItemHead reads one head of the profile. What is not well-formed
// (RFC 8949 Appendix F) is refused: the reserved additional information
// 28-30, a simple value in the two-byte form below 32 (RFC 8949 §3.3), and a
// break code where an item stands; so is an indefinite length, which the
// profile does not hold (docs/adr/0101 §3.1). Each is ErrCborMalformed.
func cborReadItemHead(c *SceCursor) (cborHead, error) {
	initial, err := cborReadBE(c, 1)
	if err != nil {
		return cborHead{}, err
	}
	h := cborHead{major: uint8(initial >> 5), info: uint8(initial & 0x1F)}
	switch {
	case h.info < 24:
		h.argument = uint64(h.info)
	case h.info == 24:
		h.argument, err = cborReadBE(c, 1)
	case h.info == 25:
		h.argument, err = cborReadBE(c, 2)
	case h.info == 26:
		h.argument, err = cborReadBE(c, 4)
	case h.info == 27:
		h.argument, err = cborReadBE(c, 8)
	default:
		return cborHead{}, ErrCborMalformed
	}
	if err != nil {
		return cborHead{}, err
	}
	if h.major == CborMajorSimple && h.info == 24 && h.argument < 32 {
		return cborHead{}, ErrCborMalformed
	}
	return h, nil
}

// CborReadHead reads one head: its major type and the value its additional
// information carries (for a string or a map, the length). An indefinite
// length, the reserved values 28–30 and a two-byte simple value below 32 are
// refused.
func CborReadHead(c *SceCursor) (uint8, uint64, error) {
	h, err := cborReadItemHead(c)
	if err != nil {
		return 0, 0, err
	}
	return h.major, h.argument, nil
}

// cborExpect answers the argument of a head of `major`; a head of another
// major type is an item the place does not take.
func cborExpect(c *SceCursor, major uint8) (uint64, error) {
	h, err := cborReadItemHead(c)
	if err != nil {
		return 0, err
	}
	if h.major != major {
		return 0, ErrCborWrongType
	}
	return h.argument, nil
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

// CborReadIntWithin reads a signed integer of least..greatest, the range of
// its type: major type 0, or major type 1 below zero. A number outside the
// range is ErrCborOutOfRange, another item ErrCborWrongType.
func CborReadIntWithin(c *SceCursor, least, greatest int64) (int64, error) {
	h, err := cborReadItemHead(c)
	if err != nil {
		return 0, err
	}
	if h.major != CborMajorUnsigned && h.major != CborMajorNegative {
		return 0, ErrCborWrongType
	}
	// An argument past math.MaxInt64 is a number past every int64.
	if h.argument > math.MaxInt64 {
		return 0, ErrCborOutOfRange
	}
	v := int64(h.argument)
	if h.major == CborMajorNegative {
		v = -1 - v
	}
	if v < least || v > greatest {
		return 0, ErrCborOutOfRange
	}
	return v, nil
}

// cborFromHalf answers the value of a half precision float's bits (IEEE 754
// binary16), exactly.
func cborFromHalf(bits uint16) float64 {
	exponent := int(bits>>10) & 0x1f
	fraction := float64(bits & 0x3ff)
	var magnitude float64
	switch {
	case exponent == 0:
		magnitude = math.Ldexp(fraction, -24)
	case exponent == 0x1f && fraction == 0:
		magnitude = math.Inf(1)
	case exponent == 0x1f:
		magnitude = math.NaN()
	default:
		magnitude = math.Ldexp(1024+fraction, exponent-25)
	}
	if bits&0x8000 != 0 {
		return -magnitude
	}
	return magnitude
}

// CborReadFloat reads a float in any of half, single and double precision,
// as its value: a variation-tolerant read (RFC 8949 §4.1). An integer, a
// simple value and every other item are ErrCborWrongType.
func CborReadFloat(c *SceCursor) (float64, error) {
	h, err := cborReadItemHead(c)
	if err != nil {
		return 0, err
	}
	if h.major != CborMajorSimple {
		return 0, ErrCborWrongType
	}
	switch h.info {
	case cborInfoHalf:
		return cborFromHalf(uint16(h.argument)), nil
	case cborInfoSingle:
		return float64(math.Float32frombits(uint32(h.argument))), nil
	case cborInfoDouble:
		return math.Float64frombits(h.argument), nil
	default:
		return 0, ErrCborWrongType
	}
}

// CborReadFloat32 reads a float32: a float in any precision whose value a
// single holds. A double that no single is exactly is ErrCborOutOfRange; a
// NaN of any precision is a NaN.
func CborReadFloat32(c *SceCursor) (float32, error) {
	x, err := CborReadFloat(c)
	if err != nil {
		return 0, err
	}
	if math.IsNaN(x) {
		return float32(math.NaN()), nil
	}
	single, ok := cborSingleOf(x)
	if !ok {
		return 0, ErrCborOutOfRange
	}
	return single, nil
}

// CborReadBool reads false or true. Every other item, null and a float among
// them, is ErrCborWrongType.
func CborReadBool(c *SceCursor) (bool, error) {
	h, err := cborReadItemHead(c)
	if err != nil {
		return false, err
	}
	if h.major != CborMajorSimple || (h.info != 20 && h.info != 21) {
		return false, ErrCborWrongType
	}
	return h.info == 21, nil
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

// CborReadArrayLen reads the head of a definite-length array: how many items
// follow. An indefinite length is refused, as a map's is.
func CborReadArrayLen(c *SceCursor) (uint64, error) {
	return cborExpect(c, CborMajorArray)
}

// CborReadMapLen reads the head of a definite-length map: how many entries
// follow.
func CborReadMapLen(c *SceCursor) (uint64, error) {
	return cborExpect(c, CborMajorMap)
}

// CborReadVariant reads the head of a union's value and names its case
// (docs/adr/0101 §2.1 and §3.3): a map, judged for exactly one entry before
// its key is read (ErrCborUnionEntries), whose key is a text string
// (ErrCborWrongType) naming one of `names` (ErrCborUnknownVariant). It
// answers the case's index in `names`; the case's record, or the empty map,
// follows.
func CborReadVariant(c *SceCursor, names []string) (int, error) {
	entries, err := CborReadMapLen(c)
	if err != nil {
		return 0, err
	}
	if entries != 1 {
		return 0, ErrCborUnionEntries
	}
	name, err := CborReadText(c, -1)
	if err != nil {
		return 0, err
	}
	for i, candidate := range names {
		if candidate == name {
			return i, nil
		}
	}
	return 0, ErrCborUnknownVariant
}

// CborSkip skips one item — the value of a key the codec does not declare —
// and everything nested in it, refusing one nested deeper than
// CborMaxSkipDepth and one the profile does not hold (an indefinite length at
// any depth).
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
