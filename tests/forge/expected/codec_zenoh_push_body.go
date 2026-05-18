// SCE-MAP: codec_zenoh_push_body:30

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_push_body

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_put"
	"example.com/sce-forge/codec_zenoh_del"
)

// CodecZenohPushBodyDefault bundles the runtime
// tag value with the catch-all body so encode can round-trip the
// observed tag back onto the wire (RFC §5.B variant primitive B1-β).
type CodecZenohPushBodyDefault struct {
	Tag uint8
	Body codec_zenoh_put.CodecZenohPut
}

// CodecZenohPushBodyVariant is a discriminated-union body for the codec's
// tag-field suffix (RFC §5.B variant primitive B1-β). Exactly one of
// the pointer fields is non-nil at a time; the active arm is the one
// that matches the current tag value.
type CodecZenohPushBodyVariant struct {
	CodecZenohPut *codec_zenoh_put.CodecZenohPut
	CodecZenohDel *codec_zenoh_del.CodecZenohDel
	Default *CodecZenohPushBodyDefault
}

// CodecZenohPushBody represents the codec frame layout.
type CodecZenohPushBody struct {
	Header uint8
	Body CodecZenohPushBodyVariant
}

// NewCodecZenohPushBody returns a CodecZenohPushBody initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecZenohPushBody().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecZenohPushBody{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity Atomic β-go.
func NewCodecZenohPushBody() *CodecZenohPushBody {
	return &CodecZenohPushBody{
		Body: CodecZenohPushBodyVariant{
			CodecZenohPut: codec_zenoh_put.NewCodecZenohPut(),
		},
	}
}

// DecodeCodecZenohPushBody decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohPushBody(cursor *codec.SceCursor) (*CodecZenohPushBody, error) {
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
	body := CodecZenohPushBodyVariant{}
	switch uint8((Header >> 0) & 0x1F) {
	case 1:
		_arm, err := codec_zenoh_put.DecodeCodecZenohPut(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohPut = _arm
	case 2:
		_arm, err := codec_zenoh_del.DecodeCodecZenohDel(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohDel = _arm
	default:
		_arm, err := codec_zenoh_put.DecodeCodecZenohPut(cursor)
		if err != nil {
			return nil, err
		}
		body.Default = &CodecZenohPushBodyDefault{
			Tag: uint8((Header >> 0) & 0x1F),
			Body: *_arm,
		}
	}
	return &CodecZenohPushBody{
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
func (s *CodecZenohPushBody) Mid() uint8 {
	return uint8((s.Header >> 0) & 0x1F)
}

func (s *CodecZenohPushBody) SetMid(v uint8) {
	const _shiftedMask uint8 = 0x1F << 0
	_val := (uint8(v) & 0x1F) << 0
	s.Header = (s.Header &^ _shiftedMask) | _val
}

func (s *CodecZenohPushBody) Rest() uint8 {
	return uint8((s.Header >> 5) & 0x07)
}

func (s *CodecZenohPushBody) SetRest(v uint8) {
	const _shiftedMask uint8 = 0x07 << 5
	_val := (uint8(v) & 0x07) << 5
	s.Header = (s.Header &^ _shiftedMask) | _val
}

// Encode serializes the CodecZenohPushBody into raw bytes.
func (s *CodecZenohPushBody) Encode() []byte {
	// Encode fixed prefix (tag field bytes are part of the prefix).
	// The tag value is read from the struct field, NOT derived from
	// the body discriminant — keeping author-set tag / body in sync
	// is the caller's responsibility (v1 keeps the layout simple).
	r := make([]byte, 0, 2)
	r = append(r, byte(s.Header))
	// Append the active arm body's encoded bytes.
	switch {
	case s.Body.CodecZenohPut != nil:
		r = append(r, s.Body.CodecZenohPut.Encode()...)
	case s.Body.CodecZenohDel != nil:
		r = append(r, s.Body.CodecZenohDel.Encode()...)
	case s.Body.Default != nil:
		r = append(r, s.Body.Default.Body.Encode()...)
	}
	return r
}
