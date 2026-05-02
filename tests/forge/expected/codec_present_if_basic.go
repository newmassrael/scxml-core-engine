// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_present_if_basic

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecPresentIfBasic represents the codec frame layout.
type CodecPresentIfBasic struct {
	Flags uint8
	Seq *uint16
}

// DecodeCodecPresentIfBasic decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecPresentIfBasic(cursor *codec.SceCursor) (*CodecPresentIfBasic, error) {
	// RFC §5.B B1-δ present-if primitive: streaming decode advances
	// the cursor per field. Gated fields use `*T` (nil = absent) and
	// allocate a stack-local `_v` only when the predicate fires; the
	// predicate test is an inline literal mask + shift on the just-
	// decoded carrier (no runtime metadata).
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
	if (Flags & 0x01) != 0 {
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
	return &CodecPresentIfBasic{
		Flags: Flags,
		Seq: Seq,
	}, nil
}

// RFC §5.B B1-γ flags primitive: per-bit accessors over the carrier
// field. Read returns a bool from `(field & mask) != 0`; write toggles
// the bit on/off without disturbing siblings on the same carrier. Wire
// layout is unchanged — the carrier still occupies its declared bytes.
func (s *CodecPresentIfBasic) HasSeq() bool {
	return (s.Flags & 0x01) != 0
}

func (s *CodecPresentIfBasic) SetHasSeq(v bool) {
	if v {
		s.Flags |= 0x01
	} else {
		s.Flags &^= 0x01
	}
}

// Encode serializes the CodecPresentIfBasic into raw bytes.
func (s *CodecPresentIfBasic) Encode() []byte {
	// RFC §5.B B1-δ encode: per-field byte append. Gated fields skip
	// the append when the pointer is nil (author keeps the carrier's
	// flag bit and the pointer's truth value in sync — same trust
	// contract as the variant primitive).
	r := make([]byte, 0, 3)
	r = append(r, s.Flags)
	if s.Seq != nil {
		_v := *s.Seq
		r = append(r, byte(_v>>8))
		r = append(r, byte(_v))
	}
	return r
}
