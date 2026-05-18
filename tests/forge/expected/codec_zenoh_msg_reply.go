// SCE-MAP: codec_zenoh_msg_reply:54

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_msg_reply

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_ext_entry"
	"example.com/sce-forge/codec_zenoh_push_body"
)

// CodecZenohMsgReply represents the codec frame layout.
type CodecZenohMsgReply struct {
	Header uint8
	Consolidation *uint8
	Extensions []codec_zenoh_ext_entry.CodecZenohExtEntry
	Body codec_zenoh_push_body.CodecZenohPushBody
}

// NewCodecZenohMsgReply returns a CodecZenohMsgReply initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecZenohMsgReply().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecZenohMsgReply{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity Atomic β-go.
func NewCodecZenohMsgReply() *CodecZenohMsgReply {
	return &CodecZenohMsgReply{
		Header: uint8(0x04),
	}
}

// DecodeCodecZenohMsgReply decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohMsgReply(cursor *codec.SceCursor) (*CodecZenohMsgReply, error) {
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
	var Body codec_zenoh_push_body.CodecZenohPushBody
	{
		_emb, err := codec_zenoh_push_body.DecodeCodecZenohPushBody(cursor)
		if err != nil {
			return nil, err
		}
		Body = *_emb
	}
	return &CodecZenohMsgReply{
		Header: Header,
		Consolidation: Consolidation,
		Extensions: Extensions,
		Body: Body,
	}, nil
}

// RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohMsgReply) Mid() uint8 {
	return uint8((s.Header >> 0) & 0x1F)
}

func (s *CodecZenohMsgReply) SetMid(v uint8) {
	const _shiftedMask uint8 = 0x1F << 0
	_val := (uint8(v) & 0x1F) << 0
	s.Header = (s.Header &^ _shiftedMask) | _val
}

func (s *CodecZenohMsgReply) C() bool {
	return (s.Header & 0x20) != 0
}

func (s *CodecZenohMsgReply) SetC(v bool) {
	if v {
		s.Header |= 0x20
	} else {
		s.Header &^= 0x20
	}
}

func (s *CodecZenohMsgReply) X() bool {
	return (s.Header & 0x40) != 0
}

func (s *CodecZenohMsgReply) SetX(v bool) {
	if v {
		s.Header |= 0x40
	} else {
		s.Header &^= 0x40
	}
}

func (s *CodecZenohMsgReply) Z() bool {
	return (s.Header & 0x80) != 0
}

func (s *CodecZenohMsgReply) SetZ(v bool) {
	if v {
		s.Header |= 0x80
	} else {
		s.Header &^= 0x80
	}
}

// Encode serializes the CodecZenohMsgReply into raw bytes.
func (s *CodecZenohMsgReply) Encode() []byte {
	// RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
	// Gated fields skip the append on nil pointer / nil slice. Per-
	// field `is_repeat` routes Repeat fields to the dedicated helper.
	// Branch fires before has_vle_fields so a codec mixing VLE +
	// present-if uses the unified encode path.
	r := make([]byte, 0, 430)
	r = append(r, s.Header)
	if s.Consolidation != nil {
		_v := *s.Consolidation
		r = append(r, _v)
	}
	for _, _e := range s.Extensions {
		r = append(r, _e.Encode()...)
	}
	r = append(r, s.Body.Encode()...)
	return r
}
