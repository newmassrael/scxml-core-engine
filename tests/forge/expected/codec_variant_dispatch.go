// SCE-MAP: codec_variant_dispatch:8

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_variant_dispatch

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_variant_session_open"
	"example.com/sce-forge/codec_variant_session_close"
)

// CodecVariantDispatchDefault bundles the runtime
// tag value with the catch-all body so encode can round-trip the
// observed tag back onto the wire (RFC §5.B variant primitive B1-β).
type CodecVariantDispatchDefault struct {
	Tag uint8
	Body codec_variant_session_close.CodecVariantSessionClose
}

// CodecVariantDispatchVariant is a discriminated-union body for the codec's
// tag-field suffix (RFC §5.B variant primitive B1-β). Exactly one of
// the pointer fields is non-nil at a time; the active arm is the one
// that matches the current tag value.
type CodecVariantDispatchVariant struct {
	CodecVariantSessionOpen *codec_variant_session_open.CodecVariantSessionOpen
	CodecVariantSessionClose *codec_variant_session_close.CodecVariantSessionClose
	Default *CodecVariantDispatchDefault
}

// CodecVariantDispatch represents the codec frame layout.
type CodecVariantDispatch struct {
	MsgId uint8
	Body CodecVariantDispatchVariant
}

// NewCodecVariantDispatch returns a CodecVariantDispatch initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecVariantDispatch().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecVariantDispatch{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity Atomic β-go.
func NewCodecVariantDispatch() *CodecVariantDispatch {
	return &CodecVariantDispatch{
		Body: CodecVariantDispatchVariant{
			CodecVariantSessionClose: &codec_variant_session_close.CodecVariantSessionClose{},
		},
	}
}

// DecodeCodecVariantDispatch decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecVariantDispatch(cursor *codec.SceCursor) (*CodecVariantDispatch, error) {
	// Decode fixed prefix (RFC §5.B variant B1-β: fields before tag suffix).
	raw, err := cursor.PeekSlice(1)
	if err != nil {
		return nil, err
	}
	MsgId := raw[0]
	if err := cursor.Advance(1); err != nil {
		return nil, err
	}
	// Dispatch on the tag field; each arm decodes its body codec from
	// the cursor. The default arm (when declared) carries the runtime
	// tag value so encode can round-trip it back onto the wire.
	body := CodecVariantDispatchVariant{}
	switch MsgId {
	case 1:
		_arm, err := codec_variant_session_open.DecodeCodecVariantSessionOpen(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecVariantSessionOpen = _arm
	case 2:
		_arm, err := codec_variant_session_close.DecodeCodecVariantSessionClose(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecVariantSessionClose = _arm
	default:
		_arm, err := codec_variant_session_close.DecodeCodecVariantSessionClose(cursor)
		if err != nil {
			return nil, err
		}
		body.Default = &CodecVariantDispatchDefault{
			Tag: MsgId,
			Body: *_arm,
		}
	}
	return &CodecVariantDispatch{
		MsgId: MsgId,
		Body: body,
	}, nil
}

// Encode writes the CodecVariantDispatch into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecVariantDispatch) Encode(w codec.SceSink) error {
	// Encode fixed prefix (tag field bytes are part of the prefix).
	if err := w.WriteBytes([]byte{ byte(s.MsgId) }); err != nil {
		return err
	}
	// Append the active arm body's encoded bytes via the same sink.
	switch {
	case s.Body.CodecVariantSessionOpen != nil:
		if err := s.Body.CodecVariantSessionOpen.Encode(w); err != nil {
			return err
		}
	case s.Body.CodecVariantSessionClose != nil:
		if err := s.Body.CodecVariantSessionClose.Encode(w); err != nil {
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
func (s *CodecVariantDispatch) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 3)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
