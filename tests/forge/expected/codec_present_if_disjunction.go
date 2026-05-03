// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_present_if_disjunction

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecPresentIfDisjunction represents the codec frame layout.
type CodecPresentIfDisjunction struct {
	Flags uint8
	Seq *uint16
}

// DecodeCodecPresentIfDisjunction decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecPresentIfDisjunction(cursor *codec.SceCursor) (*CodecPresentIfDisjunction, error) {
	// RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
	// advances the cursor per field. Gated fields use `*T` for fixed
	// (nil = absent) or `[]byte` (nil = absent) for tail/length-ref;
	// VLE gating uses `*T` like fixed. Per-field `is_repeat` routes
	// Repeat fields to the dedicated helper. Branch fires before
	// has_vle_fields so a codec mixing VLE + present-if uses the
	// unified streaming path.
	var Flags uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Flags = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var Seq *uint16
	if (Flags & 0x01) != 0 || (Flags & 0x02) != 0 {
		raw, err := cursor.PeekSlice(2)
		if err != nil {
			return nil, err
		}
		_v := uint16(raw[0])<<8 | uint16(raw[1])
		if err := cursor.Advance(2); err != nil {
			return nil, err
		}
		Seq = &_v
	}
	return &CodecPresentIfDisjunction{
		Flags: Flags,
		Seq: Seq,
	}, nil
}

// RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecPresentIfDisjunction) WantsA() bool {
	return (s.Flags & 0x01) != 0
}

func (s *CodecPresentIfDisjunction) SetWantsA(v bool) {
	if v {
		s.Flags |= 0x01
	} else {
		s.Flags &^= 0x01
	}
}

func (s *CodecPresentIfDisjunction) WantsB() bool {
	return (s.Flags & 0x02) != 0
}

func (s *CodecPresentIfDisjunction) SetWantsB(v bool) {
	if v {
		s.Flags |= 0x02
	} else {
		s.Flags &^= 0x02
	}
}

// Encode serializes the CodecPresentIfDisjunction into raw bytes.
func (s *CodecPresentIfDisjunction) Encode() []byte {
	// RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
	// Gated fields skip the append on nil pointer / nil slice. Per-
	// field `is_repeat` routes Repeat fields to the dedicated helper.
	// Branch fires before has_vle_fields so a codec mixing VLE +
	// present-if uses the unified encode path.
	r := make([]byte, 0, 3)
	r = append(r, s.Flags)
	if s.Seq != nil {
		_v := *s.Seq
		r = append(r, byte(_v>>8))
		r = append(r, byte(_v))
	}
	return r
}
