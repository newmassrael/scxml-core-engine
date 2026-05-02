// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_present_if_tail

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecPresentIfTail represents the codec frame layout.
type CodecPresentIfTail struct {
	Flags uint8
	Payload []byte
}

// DecodeCodecPresentIfTail decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecPresentIfTail(cursor *codec.SceCursor) (*CodecPresentIfTail, error) {
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
	var Payload []byte
	if (Flags & 0x01) != 0 {
		_n := cursor.Remaining()
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Payload = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	return &CodecPresentIfTail{
		Flags: Flags,
		Payload: Payload,
	}, nil
}

// RFC §5.B B1-γ flags primitive: per-bit accessors over the carrier
// field. Read returns a bool from `(field & mask) != 0`; write toggles
// the bit on/off without disturbing siblings on the same carrier. Wire
// layout is unchanged — the carrier still occupies its declared bytes.
func (s *CodecPresentIfTail) HasPayload() bool {
	return (s.Flags & 0x01) != 0
}

func (s *CodecPresentIfTail) SetHasPayload(v bool) {
	if v {
		s.Flags |= 0x01
	} else {
		s.Flags &^= 0x01
	}
}

// Encode serializes the CodecPresentIfTail into raw bytes.
func (s *CodecPresentIfTail) Encode() []byte {
	// RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
	// Gated fields skip the append on nil pointer / nil slice. Per-
	// field `is_repeat` routes Repeat fields to the dedicated helper.
	// Branch fires before has_vle_fields so a codec mixing VLE +
	// present-if uses the unified encode path.
	r := make([]byte, 0, 65)
	r = append(r, s.Flags)
	if s.Payload != nil {
		r = append(r, s.Payload...)
	}
	return r
}
