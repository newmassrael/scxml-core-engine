// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_query

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_ext_entry"
)

// CodecZenohQuery represents the codec frame layout.
type CodecZenohQuery struct {
	Header uint8
	Consolidation *uint8
	ParametersLen *uint64
	Parameters []byte
	Extensions []codec_zenoh_ext_entry.CodecZenohExtEntry
}

// DecodeCodecZenohQuery decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohQuery(cursor *codec.SceCursor) (*CodecZenohQuery, error) {
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
	var Consolidation *uint8
	if (Header & 0x20) != 0 {
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		_v := raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
		Consolidation = &_v
	}
	var ParametersLen *uint64
	if (Header & 0x40) != 0 {
		_v, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
		ParametersLen = &_v
	}
	var Parameters []byte
	if (Header & 0x40) != 0 {
		_n := int(*ParametersLen)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Parameters = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	var Extensions []codec_zenoh_ext_entry.CodecZenohExtEntry
	if (Header & 0x80) != 0 {
		Extensions = make([]codec_zenoh_ext_entry.CodecZenohExtEntry, 0, 8)
		for _i := 0; _i < int(8); _i++ {
			if cursor.Remaining() == 0 {
				break
			}
			_elem, err := codec_zenoh_ext_entry.DecodeCodecZenohExtEntry(cursor)
			if err != nil {
				return nil, err
			}
			Extensions = append(Extensions, *_elem)
		}
		if cursor.Remaining() > 0 {
			return nil, codec.ErrTlvChainOverflow
		}
	}
	return &CodecZenohQuery{
		Header: Header,
		Consolidation: Consolidation,
		ParametersLen: ParametersLen,
		Parameters: Parameters,
		Extensions: Extensions,
	}, nil
}

// RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohQuery) Mid() uint8 {
	return uint8((s.Header >> 0) & 0x1F)
}

func (s *CodecZenohQuery) SetMid(v uint8) {
	const _shiftedMask uint8 = 0x1F << 0
	_val := (uint8(v) & 0x1F) << 0
	s.Header = (s.Header &^ _shiftedMask) | _val
}

func (s *CodecZenohQuery) C() bool {
	return (s.Header & 0x20) != 0
}

func (s *CodecZenohQuery) SetC(v bool) {
	if v {
		s.Header |= 0x20
	} else {
		s.Header &^= 0x20
	}
}

func (s *CodecZenohQuery) P() bool {
	return (s.Header & 0x40) != 0
}

func (s *CodecZenohQuery) SetP(v bool) {
	if v {
		s.Header |= 0x40
	} else {
		s.Header &^= 0x40
	}
}

func (s *CodecZenohQuery) Z() bool {
	return (s.Header & 0x80) != 0
}

func (s *CodecZenohQuery) SetZ(v bool) {
	if v {
		s.Header |= 0x80
	} else {
		s.Header &^= 0x80
	}
}

// Encode serializes the CodecZenohQuery into raw bytes.
func (s *CodecZenohQuery) Encode() []byte {
	// RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
	// Gated fields skip the append on nil pointer / nil slice. Per-
	// field `is_repeat` routes Repeat fields to the dedicated helper.
	// Branch fires before has_vle_fields so a codec mixing VLE +
	// present-if uses the unified encode path.
	r := make([]byte, 0, 612)
	r = append(r, s.Header)
	if s.Consolidation != nil {
		_v := *s.Consolidation
		r = append(r, _v)
	}
	if s.ParametersLen != nil {
		_v := *s.ParametersLen
	{
		_w := uint64(_v)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	}
	if s.Parameters != nil {
		r = append(r, s.Parameters...)
	}
	for _, _e := range s.Extensions {
		r = append(r, _e.Encode()...)
	}
	return r
}
