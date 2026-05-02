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

// CodecVariantDispatchBody is a discriminated-union body for the codec's
// tag-field suffix (RFC §5.B variant primitive B1-β). Exactly one of
// the pointer fields is non-nil at a time; the active arm is the one
// that matches the current tag value.
type CodecVariantDispatchBody struct {
	CodecVariantSessionOpen *codec_variant_session_open.CodecVariantSessionOpen
	CodecVariantSessionClose *codec_variant_session_close.CodecVariantSessionClose
	Default *CodecVariantDispatchDefault
}

// CodecVariantDispatch represents the codec frame layout.
type CodecVariantDispatch struct {
	MsgId uint8
	Body CodecVariantDispatchBody
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
	body := CodecVariantDispatchBody{}
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

// Encode serializes the CodecVariantDispatch into raw bytes.
func (s *CodecVariantDispatch) Encode() []byte {
	// Encode fixed prefix (tag field bytes are part of the prefix).
	// The tag value is read from the struct field, NOT derived from
	// the body discriminant — keeping author-set tag / body in sync
	// is the caller's responsibility (v1 keeps the layout simple).
	r := make([]byte, 0, 1)
	r = append(r, byte(s.MsgId))
	// Append the active arm body's encoded bytes.
	switch {
	case s.Body.CodecVariantSessionOpen != nil:
		r = append(r, s.Body.CodecVariantSessionOpen.Encode()...)
	case s.Body.CodecVariantSessionClose != nil:
		r = append(r, s.Body.CodecVariantSessionClose.Encode()...)
	case s.Body.Default != nil:
		r = append(r, s.Body.Default.Body.Encode()...)
	}
	return r
}
