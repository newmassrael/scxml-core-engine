// SCE-MAP: codec_zenoh_interest_body:56

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_interest_body

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_wireexpr"
)

// CodecZenohInterestBody represents the codec frame layout.
type CodecZenohInterestBody struct {
	Header uint8
	Keyexpr *codec_zenoh_wireexpr.CodecZenohWireexpr
}

// DecodeCodecZenohInterestBody decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohInterestBody(cursor *codec.SceCursor) (*CodecZenohInterestBody, error) {
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
	var Keyexpr *codec_zenoh_wireexpr.CodecZenohWireexpr
	if (Header & 0x10) != 0 {
		_emb, err := codec_zenoh_wireexpr.DecodeCodecZenohWireexpr(cursor, byte((Header >> 5) & 0x1))
		if err != nil {
			return nil, err
		}
		Keyexpr = _emb
	}
	return &CodecZenohInterestBody{
		Header: Header,
		Keyexpr: Keyexpr,
	}, nil
}

// RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohInterestBody) Keyexprs() bool {
	return (s.Header & 0x01) != 0
}

func (s *CodecZenohInterestBody) SetKeyexprs(v bool) {
	if v {
		s.Header |= 0x01
	} else {
		s.Header &^= 0x01
	}
}

func (s *CodecZenohInterestBody) Subscribers() bool {
	return (s.Header & 0x02) != 0
}

func (s *CodecZenohInterestBody) SetSubscribers(v bool) {
	if v {
		s.Header |= 0x02
	} else {
		s.Header &^= 0x02
	}
}

func (s *CodecZenohInterestBody) Queryables() bool {
	return (s.Header & 0x04) != 0
}

func (s *CodecZenohInterestBody) SetQueryables(v bool) {
	if v {
		s.Header |= 0x04
	} else {
		s.Header &^= 0x04
	}
}

func (s *CodecZenohInterestBody) Tokens() bool {
	return (s.Header & 0x08) != 0
}

func (s *CodecZenohInterestBody) SetTokens(v bool) {
	if v {
		s.Header |= 0x08
	} else {
		s.Header &^= 0x08
	}
}

func (s *CodecZenohInterestBody) Restricted() bool {
	return (s.Header & 0x10) != 0
}

func (s *CodecZenohInterestBody) SetRestricted(v bool) {
	if v {
		s.Header |= 0x10
	} else {
		s.Header &^= 0x10
	}
}

func (s *CodecZenohInterestBody) N() bool {
	return (s.Header & 0x20) != 0
}

func (s *CodecZenohInterestBody) SetN(v bool) {
	if v {
		s.Header |= 0x20
	} else {
		s.Header &^= 0x20
	}
}

func (s *CodecZenohInterestBody) M() bool {
	return (s.Header & 0x40) != 0
}

func (s *CodecZenohInterestBody) SetM(v bool) {
	if v {
		s.Header |= 0x40
	} else {
		s.Header &^= 0x40
	}
}

func (s *CodecZenohInterestBody) Aggregate() bool {
	return (s.Header & 0x80) != 0
}

func (s *CodecZenohInterestBody) SetAggregate(v bool) {
	if v {
		s.Header |= 0x80
	} else {
		s.Header &^= 0x80
	}
}

// Encode serializes the CodecZenohInterestBody into raw bytes.
func (s *CodecZenohInterestBody) Encode() []byte {
	// RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
	// Gated fields skip the append on nil pointer / nil slice. Per-
	// field `is_repeat` routes Repeat fields to the dedicated helper.
	// Branch fires before has_vle_fields so a codec mixing VLE +
	// present-if uses the unified encode path.
	r := make([]byte, 0, 257)
	r = append(r, s.Header)
	if s.Keyexpr != nil {
		r = append(r, s.Keyexpr.Encode(byte((s.Header >> 5) & 0x1))...)
	}
	return r
}
