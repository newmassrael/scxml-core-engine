// SCE-MAP: codec_zenoh_err:54

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_err

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_encoding"
	"example.com/sce-forge/codec_zenoh_ext_entry"
)

// CodecZenohErr represents the codec frame layout.
type CodecZenohErr struct {
	Header uint8
	Encoding *codec_zenoh_encoding.CodecZenohEncoding
	Extensions []codec_zenoh_ext_entry.CodecZenohExtEntry
	PayloadLen uint64
	Payload []byte
}

// NewCodecZenohErr returns a CodecZenohErr initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecZenohErr().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecZenohErr{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity Atomic β-go.
func NewCodecZenohErr() *CodecZenohErr {
	return &CodecZenohErr{
		Header: uint8(0x05),
	}
}

// DecodeCodecZenohErr decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohErr(cursor *codec.SceCursor) (*CodecZenohErr, error) {
	// RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
	// advances the cursor per field. Gated fields use `*T` for fixed
	// (nil = absent) or `[]byte` (nil = absent) for tail/length-ref;
	// VLE gating uses `*T` like fixed. Per-field `is_repeat` routes
	// Repeat fields to the dedicated helper. Branch fires before
	// has_vle_fields so a codec mixing VLE + present-if uses the
	// unified streaming path.
	var Header uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Header = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var Encoding *codec_zenoh_encoding.CodecZenohEncoding
	if (Header & 0x40) != 0 {
		_emb, err := codec_zenoh_encoding.DecodeCodecZenohEncoding(cursor)
		if err != nil {
			return nil, err
		}
		Encoding = _emb
	}
	var Extensions []codec_zenoh_ext_entry.CodecZenohExtEntry
	if (Header & 0x80) != 0 {
		Extensions = make([]codec_zenoh_ext_entry.CodecZenohExtEntry, 0, 4)
		for _i := 0; _i < int(4); _i++ {
			if cursor.Remaining() == 0 {
				break
			}
			_elem, err := codec_zenoh_ext_entry.DecodeCodecZenohExtEntry(cursor)
			if err != nil {
				return nil, err
			}
			_continue := _elem.Z()
			Extensions = append(Extensions, *_elem)
			if !_continue {
				break
			}
		}
	}
	PayloadLen, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var Payload []byte
	{
		_n := int(PayloadLen)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Payload = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	return &CodecZenohErr{
		Header: Header,
		Encoding: Encoding,
		Extensions: Extensions,
		PayloadLen: PayloadLen,
		Payload: Payload,
	}, nil
}

// RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohErr) Mid() uint8 {
	return uint8((s.Header >> 0) & 0x1F)
}

func (s *CodecZenohErr) SetMid(v uint8) {
	const _shiftedMask uint8 = 0x1F << 0
	_val := (uint8(v) & 0x1F) << 0
	s.Header = (s.Header &^ _shiftedMask) | _val
}

func (s *CodecZenohErr) X() bool {
	return (s.Header & 0x20) != 0
}

func (s *CodecZenohErr) SetX(v bool) {
	if v {
		s.Header |= 0x20
	} else {
		s.Header &^= 0x20
	}
}

func (s *CodecZenohErr) E() bool {
	return (s.Header & 0x40) != 0
}

func (s *CodecZenohErr) SetE(v bool) {
	if v {
		s.Header |= 0x40
	} else {
		s.Header &^= 0x40
	}
}

func (s *CodecZenohErr) Z() bool {
	return (s.Header & 0x80) != 0
}

func (s *CodecZenohErr) SetZ(v bool) {
	if v {
		s.Header |= 0x80
	} else {
		s.Header &^= 0x80
	}
}

// Encode serializes the CodecZenohErr into raw bytes.
func (s *CodecZenohErr) Encode() []byte {
	// RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
	// Gated fields skip the append on nil pointer / nil slice. Per-
	// field `is_repeat` routes Repeat fields to the dedicated helper.
	// Branch fires before has_vle_fields so a codec mixing VLE +
	// present-if uses the unified encode path.
	r := make([]byte, 0, 695)
	r = append(r, s.Header)
	if s.Encoding != nil {
		r = append(r, s.Encoding.Encode()...)
	}
	for _, _e := range s.Extensions {
		r = append(r, _e.Encode()...)
	}
	{
		_w := uint64(s.PayloadLen)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	r = append(r, s.Payload...)
	return r
}
