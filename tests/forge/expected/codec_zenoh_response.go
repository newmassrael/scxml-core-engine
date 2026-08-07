// SCE-MAP: codec_zenoh_response:75

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_response

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"unicode/utf8"
	"example.com/sce-forge/codec_zenoh_ext_entry"
	"example.com/sce-forge/codec_zenoh_reply"
	"example.com/sce-forge/codec_zenoh_err"
)

// CodecZenohResponseDefault bundles the runtime
// tag value with the catch-all body so encode can round-trip the
// observed tag back onto the wire (RFC §synth-5-B variant primitive).
type CodecZenohResponseDefault struct {
	Tag uint8
	Body codec_zenoh_reply.CodecZenohReply
}

// CodecZenohResponseVariant is a discriminated-union body for the codec's
// tag-field suffix (RFC §synth-5-B variant primitive). Exactly one of
// the pointer fields is non-nil at a time; the active arm is the one
// that matches the current tag value.
type CodecZenohResponseVariant struct {
	CodecZenohReply *codec_zenoh_reply.CodecZenohReply
	CodecZenohErr *codec_zenoh_err.CodecZenohErr
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

// NewCodecZenohResponse returns a CodecZenohResponse initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecZenohResponse().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecZenohResponse{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecZenohResponse() *CodecZenohResponse {
	return &CodecZenohResponse{
		Header: uint8(0x1b),
		Body: CodecZenohResponseVariant{
			CodecZenohReply: codec_zenoh_reply.NewCodecZenohReply(),
		},
	}
}

// DecodeCodecZenohResponse decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohResponse(cursor *codec.SceCursor) (*CodecZenohResponse, error) {
	// RFC §synth-5-B peek-byte / streaming-prefix:
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
		_more := false
		for _i := 0; _i < int(4); _i++ {
			if cursor.Remaining() == 0 {
				break
			}
			_elem, err := codec_zenoh_ext_entry.DecodeCodecZenohExtEntry(cursor)
			if err != nil {
				return nil, err
			}
			_more = _elem.Z()
			Extensions = append(Extensions, *_elem)
			if !_more {
				break
			}
		}
		if _more && cursor.Remaining() == 0 {
			return nil, codec.ErrNeedMoreBytes
		}
		if _more {
			return nil, codec.ErrTlvChainOverflow
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
		_arm, err := codec_zenoh_reply.DecodeCodecZenohReply(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohReply = _arm
	case 5:
		_arm, err := codec_zenoh_err.DecodeCodecZenohErr(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohErr = _arm
	default:
		_arm, err := codec_zenoh_reply.DecodeCodecZenohReply(cursor)
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

// RFC §synth-5-B flags primitive: per-bit-range accessors over
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

// Encode writes the CodecZenohResponse into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohResponse) Encode(w codec.SceSink) error {
	// RFC §synth-5-B peek-byte / streaming-prefix.
	if err := w.WriteBytes([]byte{ s.Header }); err != nil {
		return err
	}
	if err := codec.WriteVLEU64(w, uint64(s.RequestId)); err != nil {
		return err
	}
	if err := codec.WriteVLEU32(w, uint32(s.KeyId)); err != nil {
		return err
	}
	if s.SuffixLen != nil {
		_v := *s.SuffixLen
	if err := codec.WriteVLEU64(w, uint64(_v)); err != nil {
		return err
	}
	}
	if s.Suffix != nil {
		if err := w.WriteBytes([]byte(*s.Suffix)); err != nil {
			return err
		}
	}
	for _i := range s.Extensions {
		if err := s.Extensions[_i].Encode(w); err != nil {
			return err
		}
	}
	// Append the active arm body's encoded bytes via the same sink.
	switch {
	case s.Body.CodecZenohReply != nil:
		if err := s.Body.CodecZenohReply.Encode(w); err != nil {
			return err
		}
	case s.Body.CodecZenohErr != nil:
		if err := s.Body.CodecZenohErr.Encode(w); err != nil {
			return err
		}
	case s.Body.Default != nil:
		if err := s.Body.Default.Body.Encode(w); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohResponse) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 970)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
