// SCE-MAP: codec_zenoh_request:73

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_request

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_wireexpr"
	"example.com/sce-forge/codec_zenoh_ext_entry"
	"example.com/sce-forge/codec_zenoh_msg_put"
	"example.com/sce-forge/codec_zenoh_msg_del"
	"example.com/sce-forge/codec_zenoh_query"
)

// CodecZenohRequestDefault bundles the runtime
// tag value with the catch-all body so encode can round-trip the
// observed tag back onto the wire (RFC §5.B variant primitive B1-β).
type CodecZenohRequestDefault struct {
	Tag uint8
	Body codec_zenoh_query.CodecZenohQuery
}

// CodecZenohRequestVariant is a discriminated-union body for the codec's
// tag-field suffix (RFC §5.B variant primitive B1-β). Exactly one of
// the pointer fields is non-nil at a time; the active arm is the one
// that matches the current tag value.
type CodecZenohRequestVariant struct {
	CodecZenohMsgPut *codec_zenoh_msg_put.CodecZenohMsgPut
	CodecZenohMsgDel *codec_zenoh_msg_del.CodecZenohMsgDel
	CodecZenohQuery *codec_zenoh_query.CodecZenohQuery
	Default *CodecZenohRequestDefault
}

// CodecZenohRequest represents the codec frame layout.
type CodecZenohRequest struct {
	Header uint8
	Rid uint64
	Keyexpr codec_zenoh_wireexpr.CodecZenohWireexpr
	Extensions []codec_zenoh_ext_entry.CodecZenohExtEntry
	Body CodecZenohRequestVariant
}

// NewCodecZenohRequest returns a CodecZenohRequest initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecZenohRequest().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecZenohRequest{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity Atomic β-go.
func NewCodecZenohRequest() *CodecZenohRequest {
	return &CodecZenohRequest{
		Header: uint8(0x1c),
		Body: CodecZenohRequestVariant{
			CodecZenohMsgPut: codec_zenoh_msg_put.NewCodecZenohMsgPut(),
		},
	}
}

// DecodeCodecZenohRequest decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohRequest(cursor *codec.SceCursor) (*CodecZenohRequest, error) {
	// RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
	// streaming prefix decode (variable-length fields supported via
	// per-field present_if/tlv-chain/embed/repeat helpers). Peek-byte
	// mode additionally peeks the cursor's next byte for variant tag
	// without advancing — arm body decoder reads it as own header.
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
	Rid, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var Keyexpr codec_zenoh_wireexpr.CodecZenohWireexpr
	{
		_emb, err := codec_zenoh_wireexpr.DecodeCodecZenohWireexpr(cursor, byte((Header >> 5) & 0x1))
		if err != nil {
			return nil, err
		}
		Keyexpr = *_emb
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
	_peekSlice, err := cursor.PeekSlice(1)
	if err != nil {
		return nil, err
	}
	_peek := _peekSlice[0]
	// Dispatch on the tag field; each arm decodes its body codec from
	// the cursor. The default arm (when declared) carries the runtime
	// tag value so encode can round-trip it back onto the wire.
	body := CodecZenohRequestVariant{}
	switch uint8((_peek >> 0) & 0x1F) {
	case 1:
		_arm, err := codec_zenoh_msg_put.DecodeCodecZenohMsgPut(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohMsgPut = _arm
	case 2:
		_arm, err := codec_zenoh_msg_del.DecodeCodecZenohMsgDel(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohMsgDel = _arm
	case 3:
		_arm, err := codec_zenoh_query.DecodeCodecZenohQuery(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohQuery = _arm
	default:
		_arm, err := codec_zenoh_query.DecodeCodecZenohQuery(cursor)
		if err != nil {
			return nil, err
		}
		body.Default = &CodecZenohRequestDefault{
			Tag: uint8((_peek >> 0) & 0x1F),
			Body: *_arm,
		}
	}
	return &CodecZenohRequest{
		Header: Header,
		Rid: Rid,
		Keyexpr: Keyexpr,
		Extensions: Extensions,
		Body: body,
	}, nil
}

// RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohRequest) Mid() uint8 {
	return uint8((s.Header >> 0) & 0x1F)
}

func (s *CodecZenohRequest) SetMid(v uint8) {
	const _shiftedMask uint8 = 0x1F << 0
	_val := (uint8(v) & 0x1F) << 0
	s.Header = (s.Header &^ _shiftedMask) | _val
}

func (s *CodecZenohRequest) N() bool {
	return (s.Header & 0x20) != 0
}

func (s *CodecZenohRequest) SetN(v bool) {
	if v {
		s.Header |= 0x20
	} else {
		s.Header &^= 0x20
	}
}

func (s *CodecZenohRequest) M() bool {
	return (s.Header & 0x40) != 0
}

func (s *CodecZenohRequest) SetM(v bool) {
	if v {
		s.Header |= 0x40
	} else {
		s.Header &^= 0x40
	}
}

func (s *CodecZenohRequest) Z() bool {
	return (s.Header & 0x80) != 0
}

func (s *CodecZenohRequest) SetZ(v bool) {
	if v {
		s.Header |= 0x80
	} else {
		s.Header &^= 0x80
	}
}

// Encode serializes the CodecZenohRequest into raw bytes.
func (s *CodecZenohRequest) Encode() []byte {
	// RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
	// streaming prefix encode. Peek-byte mode: arm body's encode
	// prepends its own header byte (which the decoder peeked); no
	// separate tag byte here. Streaming-prefix mode (own-field):
	// carrier is part of the prefix fields and emits via the same
	// per-field path.
	r := make([]byte, 0, 1218)
	r = append(r, s.Header)
	{
		_w := uint64(s.Rid)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	r = append(r, s.Keyexpr.Encode(byte((s.Header >> 5) & 0x1))...)
	for _, _e := range s.Extensions {
		r = append(r, _e.Encode()...)
	}
	// Append the active arm body's encoded bytes.
	switch {
	case s.Body.CodecZenohMsgPut != nil:
		r = append(r, s.Body.CodecZenohMsgPut.Encode()...)
	case s.Body.CodecZenohMsgDel != nil:
		r = append(r, s.Body.CodecZenohMsgDel.Encode()...)
	case s.Body.CodecZenohQuery != nil:
		r = append(r, s.Body.CodecZenohQuery.Encode()...)
	case s.Body.Default != nil:
		r = append(r, s.Body.Default.Body.Encode()...)
	}
	return r
}
