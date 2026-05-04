// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_response

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"unicode/utf8"
	"example.com/sce-forge/codec_zenoh_ext_entry"
	"example.com/sce-forge/codec_zenoh_msg_reply"
	"example.com/sce-forge/codec_zenoh_msg_err"
)

// CodecZenohResponseDefault bundles the runtime
// tag value with the catch-all body so encode can round-trip the
// observed tag back onto the wire (RFC §5.B variant primitive B1-β).
type CodecZenohResponseDefault struct {
	Tag uint8
	Body codec_zenoh_msg_reply.CodecZenohMsgReply
}

// CodecZenohResponseVariant is a discriminated-union body for the codec's
// tag-field suffix (RFC §5.B variant primitive B1-β). Exactly one of
// the pointer fields is non-nil at a time; the active arm is the one
// that matches the current tag value.
type CodecZenohResponseVariant struct {
	CodecZenohMsgReply *codec_zenoh_msg_reply.CodecZenohMsgReply
	CodecZenohMsgErr *codec_zenoh_msg_err.CodecZenohMsgErr
	Default *CodecZenohResponseDefault
}

// CodecZenohResponse represents the codec frame layout.
type CodecZenohResponse struct {
	Header uint8
	RequestId uint64
	KeyId uint32
	SuffixLen *uint64
	Suffix *string
	Extensions []codec_zenoh_ext_entry.CodecZenohExtEntry
	Body CodecZenohResponseVariant
}

// DecodeCodecZenohResponse decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohResponse(cursor *codec.SceCursor) (*CodecZenohResponse, error) {
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
	RequestId, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	KeyId, err := cursor.ReadVLEU32()
	if err != nil { return nil, err }
	var SuffixLen *uint64
	if (Header & 0x20) != 0 {
		_v, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
		SuffixLen = &_v
	}
	var Suffix *string
	if (Header & 0x20) != 0 {
		_n := int(*SuffixLen)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		if !utf8.Valid(raw) {
			return nil, codec.ErrInvalidUTF8
		}
		_v := string(raw)
		Suffix = &_v
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
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
	body := CodecZenohResponseVariant{}
	switch uint8((_peek >> 0) & 0x1F) {
	case 4:
		_arm, err := codec_zenoh_msg_reply.DecodeCodecZenohMsgReply(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohMsgReply = _arm
	case 5:
		_arm, err := codec_zenoh_msg_err.DecodeCodecZenohMsgErr(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohMsgErr = _arm
	default:
		_arm, err := codec_zenoh_msg_reply.DecodeCodecZenohMsgReply(cursor)
		if err != nil {
			return nil, err
		}
		body.Default = &CodecZenohResponseDefault{
			Tag: uint8((_peek >> 0) & 0x1F),
			Body: *_arm,
		}
	}
	return &CodecZenohResponse{
		Header: Header,
		RequestId: RequestId,
		KeyId: KeyId,
		SuffixLen: SuffixLen,
		Suffix: Suffix,
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
func (s *CodecZenohResponse) Mid() uint8 {
	return uint8((s.Header >> 0) & 0x1F)
}

func (s *CodecZenohResponse) SetMid(v uint8) {
	const _shiftedMask uint8 = 0x1F << 0
	_val := (uint8(v) & 0x1F) << 0
	s.Header = (s.Header &^ _shiftedMask) | _val
}

func (s *CodecZenohResponse) N() bool {
	return (s.Header & 0x20) != 0
}

func (s *CodecZenohResponse) SetN(v bool) {
	if v {
		s.Header |= 0x20
	} else {
		s.Header &^= 0x20
	}
}

func (s *CodecZenohResponse) M() bool {
	return (s.Header & 0x40) != 0
}

func (s *CodecZenohResponse) SetM(v bool) {
	if v {
		s.Header |= 0x40
	} else {
		s.Header &^= 0x40
	}
}

func (s *CodecZenohResponse) Z() bool {
	return (s.Header & 0x80) != 0
}

func (s *CodecZenohResponse) SetZ(v bool) {
	if v {
		s.Header |= 0x80
	} else {
		s.Header &^= 0x80
	}
}

// Encode serializes the CodecZenohResponse into raw bytes.
func (s *CodecZenohResponse) Encode() []byte {
	// RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
	// streaming prefix encode. Peek-byte mode: arm body's encode
	// prepends its own header byte (which the decoder peeked); no
	// separate tag byte here. Streaming-prefix mode (own-field):
	// carrier is part of the prefix fields and emits via the same
	// per-field path.
	r := make([]byte, 0, 977)
	r = append(r, s.Header)
	{
		_w := uint64(s.RequestId)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	{
		_w := uint64(s.KeyId)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	if s.SuffixLen != nil {
		_v := *s.SuffixLen
	{
		_w := uint64(_v)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	}
	if s.Suffix != nil {
		r = append(r, []byte(*s.Suffix)...)
	}
	for _, _e := range s.Extensions {
		r = append(r, _e.Encode()...)
	}
	// Append the active arm body's encoded bytes.
	switch {
	case s.Body.CodecZenohMsgReply != nil:
		r = append(r, s.Body.CodecZenohMsgReply.Encode()...)
	case s.Body.CodecZenohMsgErr != nil:
		r = append(r, s.Body.CodecZenohMsgErr.Encode()...)
	case s.Body.Default != nil:
		r = append(r, s.Body.Default.Body.Encode()...)
	}
	return r
}
