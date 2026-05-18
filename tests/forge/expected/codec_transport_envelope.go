// SCE-MAP: codec_transport_envelope:69

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_transport_envelope

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_init_body"
	"example.com/sce-forge/codec_zenoh_open_body"
	"example.com/sce-forge/codec_zenoh_close"
	"example.com/sce-forge/codec_zenoh_keep_alive"
	"example.com/sce-forge/codec_zenoh_frame"
	"example.com/sce-forge/codec_zenoh_fragment"
	"example.com/sce-forge/codec_zenoh_join"
)

// CodecTransportEnvelopeDefault bundles the runtime
// tag value with the catch-all body so encode can round-trip the
// observed tag back onto the wire (RFC §5.B variant primitive B1-β).
type CodecTransportEnvelopeDefault struct {
	Tag uint8
	Body codec_zenoh_close.CodecZenohClose
}

// CodecTransportEnvelopeVariant is a discriminated-union body for the codec's
// tag-field suffix (RFC §5.B variant primitive B1-β). Exactly one of
// the pointer fields is non-nil at a time; the active arm is the one
// that matches the current tag value.
type CodecTransportEnvelopeVariant struct {
	CodecZenohInitBody *codec_zenoh_init_body.CodecZenohInitBody
	CodecZenohOpenBody *codec_zenoh_open_body.CodecZenohOpenBody
	CodecZenohClose *codec_zenoh_close.CodecZenohClose
	CodecZenohKeepAlive *codec_zenoh_keep_alive.CodecZenohKeepAlive
	CodecZenohFrame *codec_zenoh_frame.CodecZenohFrame
	CodecZenohFragment *codec_zenoh_fragment.CodecZenohFragment
	CodecZenohJoin *codec_zenoh_join.CodecZenohJoin
	Default *CodecTransportEnvelopeDefault
}

// CodecTransportEnvelope represents the codec frame layout.
type CodecTransportEnvelope struct {
	Header uint8
	Body CodecTransportEnvelopeVariant
}

// NewCodecTransportEnvelope returns a CodecTransportEnvelope initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecTransportEnvelope().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecTransportEnvelope{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity Atomic β-go.
func NewCodecTransportEnvelope() *CodecTransportEnvelope {
	return &CodecTransportEnvelope{
		Body: CodecTransportEnvelopeVariant{
			CodecZenohClose: codec_zenoh_close.NewCodecZenohClose(),
		},
	}
}

// DecodeCodecTransportEnvelope decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecTransportEnvelope(cursor *codec.SceCursor) (*CodecTransportEnvelope, error) {
	// Decode fixed prefix (RFC §5.B variant B1-β: fields before tag suffix).
	raw, err := cursor.PeekSlice(1)
	if err != nil {
		return nil, err
	}
	Header := raw[0]
	if err := cursor.Advance(1); err != nil {
		return nil, err
	}
	// Dispatch on the tag field; each arm decodes its body codec from
	// the cursor. The default arm (when declared) carries the runtime
	// tag value so encode can round-trip it back onto the wire.
	body := CodecTransportEnvelopeVariant{}
	switch uint8((Header >> 0) & 0x1F) {
	case 1:
		_arm, err := codec_zenoh_init_body.DecodeCodecZenohInitBody(cursor, Header)
		if err != nil {
			return nil, err
		}
		body.CodecZenohInitBody = _arm
	case 2:
		_arm, err := codec_zenoh_open_body.DecodeCodecZenohOpenBody(cursor, Header)
		if err != nil {
			return nil, err
		}
		body.CodecZenohOpenBody = _arm
	case 3:
		_arm, err := codec_zenoh_close.DecodeCodecZenohClose(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohClose = _arm
	case 4:
		_arm, err := codec_zenoh_keep_alive.DecodeCodecZenohKeepAlive(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohKeepAlive = _arm
	case 5:
		_arm, err := codec_zenoh_frame.DecodeCodecZenohFrame(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohFrame = _arm
	case 6:
		_arm, err := codec_zenoh_fragment.DecodeCodecZenohFragment(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohFragment = _arm
	case 7:
		_arm, err := codec_zenoh_join.DecodeCodecZenohJoin(cursor, Header)
		if err != nil {
			return nil, err
		}
		body.CodecZenohJoin = _arm
	default:
		_arm, err := codec_zenoh_close.DecodeCodecZenohClose(cursor)
		if err != nil {
			return nil, err
		}
		body.Default = &CodecTransportEnvelopeDefault{
			Tag: uint8((Header >> 0) & 0x1F),
			Body: *_arm,
		}
	}
	return &CodecTransportEnvelope{
		Header: Header,
		Body: body,
	}, nil
}

// RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecTransportEnvelope) Mid() uint8 {
	return uint8((s.Header >> 0) & 0x1F)
}

func (s *CodecTransportEnvelope) SetMid(v uint8) {
	const _shiftedMask uint8 = 0x1F << 0
	_val := (uint8(v) & 0x1F) << 0
	s.Header = (s.Header &^ _shiftedMask) | _val
}

func (s *CodecTransportEnvelope) A() bool {
	return (s.Header & 0x20) != 0
}

func (s *CodecTransportEnvelope) SetA(v bool) {
	if v {
		s.Header |= 0x20
	} else {
		s.Header &^= 0x20
	}
}

func (s *CodecTransportEnvelope) S() bool {
	return (s.Header & 0x40) != 0
}

func (s *CodecTransportEnvelope) SetS(v bool) {
	if v {
		s.Header |= 0x40
	} else {
		s.Header &^= 0x40
	}
}

func (s *CodecTransportEnvelope) Z() bool {
	return (s.Header & 0x80) != 0
}

func (s *CodecTransportEnvelope) SetZ(v bool) {
	if v {
		s.Header |= 0x80
	} else {
		s.Header &^= 0x80
	}
}

// Encode serializes the CodecTransportEnvelope into raw bytes.
func (s *CodecTransportEnvelope) Encode() []byte {
	// Encode fixed prefix (tag field bytes are part of the prefix).
	// The tag value is read from the struct field, NOT derived from
	// the body discriminant — keeping author-set tag / body in sync
	// is the caller's responsibility (v1 keeps the layout simple).
	r := make([]byte, 0, 65547)
	r = append(r, byte(s.Header))
	// Append the active arm body's encoded bytes.
	switch {
	case s.Body.CodecZenohInitBody != nil:
		r = append(r, s.Body.CodecZenohInitBody.Encode(s.Header)...)
	case s.Body.CodecZenohOpenBody != nil:
		r = append(r, s.Body.CodecZenohOpenBody.Encode(s.Header)...)
	case s.Body.CodecZenohClose != nil:
		r = append(r, s.Body.CodecZenohClose.Encode()...)
	case s.Body.CodecZenohKeepAlive != nil:
		r = append(r, s.Body.CodecZenohKeepAlive.Encode()...)
	case s.Body.CodecZenohFrame != nil:
		r = append(r, s.Body.CodecZenohFrame.Encode()...)
	case s.Body.CodecZenohFragment != nil:
		r = append(r, s.Body.CodecZenohFragment.Encode()...)
	case s.Body.CodecZenohJoin != nil:
		r = append(r, s.Body.CodecZenohJoin.Encode(s.Header)...)
	case s.Body.Default != nil:
		r = append(r, s.Body.Default.Body.Encode()...)
	}
	return r
}
