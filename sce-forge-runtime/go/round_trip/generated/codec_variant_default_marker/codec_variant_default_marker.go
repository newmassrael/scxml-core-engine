// SCE-GENERATED — DO NOT EDIT
// source-hash: 9dba19024112d81b0a74dd568083d98ea9f9b26f403a3af95dfa629b72fd2464
// template-hash: 424c6f33953fe7f160097f9838fbc995a6528c2c939f5d44e179aa010bc5eec7
// generated-at: 1779070329
// SCE-MAP: codec_variant_default_marker.scxml:30

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_variant_default_marker

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/round_trip/generated/codec_default_marker_arm_a"
	"github.com/newmassrael/sce-forge-runtime/round_trip/generated/codec_default_marker_arm_b"
)

// CodecVariantDefaultMarkerDefault bundles the runtime
// tag value with the catch-all body so encode can round-trip the
// observed tag back onto the wire (RFC §5.B variant primitive B1-β).
type CodecVariantDefaultMarkerDefault struct {
	Tag uint8
	Body codec_default_marker_arm_b.CodecDefaultMarkerArmB
}

// CodecVariantDefaultMarkerVariant is a discriminated-union body for the codec's
// tag-field suffix (RFC §5.B variant primitive B1-β). Exactly one of
// the pointer fields is non-nil at a time; the active arm is the one
// that matches the current tag value.
type CodecVariantDefaultMarkerVariant struct {
	CodecDefaultMarkerArmA *codec_default_marker_arm_a.CodecDefaultMarkerArmA
	CodecDefaultMarkerArmB *codec_default_marker_arm_b.CodecDefaultMarkerArmB
	Default *CodecVariantDefaultMarkerDefault
}

// CodecVariantDefaultMarker represents the codec frame layout.
type CodecVariantDefaultMarker struct {
	Body CodecVariantDefaultMarkerVariant
}

// NewCodecVariantDefaultMarker returns a CodecVariantDefaultMarker initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecVariantDefaultMarker().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecVariantDefaultMarker{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity Atomic β-go.
func NewCodecVariantDefaultMarker() *CodecVariantDefaultMarker {
	return &CodecVariantDefaultMarker{
		Body: CodecVariantDefaultMarkerVariant{
			CodecDefaultMarkerArmB: codec_default_marker_arm_b.NewCodecDefaultMarkerArmB(),
		},
	}
}

// DecodeCodecVariantDefaultMarker decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecVariantDefaultMarker(cursor *codec.SceCursor) (*CodecVariantDefaultMarker, error) {
	// RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
	// streaming prefix decode (variable-length fields supported via
	// per-field present_if/tlv-chain/embed/repeat helpers). Peek-byte
	// mode additionally peeks the cursor's next byte for variant tag
	// without advancing — arm body decoder reads it as own header.
	_peekSlice, err := cursor.PeekSlice(1)
	if err != nil {
		return nil, err
	}
	_peek := _peekSlice[0]
	// Dispatch on the tag field; each arm decodes its body codec from
	// the cursor. The default arm (when declared) carries the runtime
	// tag value so encode can round-trip it back onto the wire.
	body := CodecVariantDefaultMarkerVariant{}
	switch uint8((_peek >> 0) & 0x03) {
	case 1:
		_arm, err := codec_default_marker_arm_a.DecodeCodecDefaultMarkerArmA(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecDefaultMarkerArmA = _arm
	case 2:
		_arm, err := codec_default_marker_arm_b.DecodeCodecDefaultMarkerArmB(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecDefaultMarkerArmB = _arm
	default:
		_arm, err := codec_default_marker_arm_b.DecodeCodecDefaultMarkerArmB(cursor)
		if err != nil {
			return nil, err
		}
		body.Default = &CodecVariantDefaultMarkerDefault{
			Tag: uint8((_peek >> 0) & 0x03),
			Body: *_arm,
		}
	}
	return &CodecVariantDefaultMarker{
		Body: body,
	}, nil
}

// Encode serializes the CodecVariantDefaultMarker into raw bytes.
func (s *CodecVariantDefaultMarker) Encode() []byte {
	// RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
	// streaming prefix encode. Peek-byte mode: arm body's encode
	// prepends its own header byte (which the decoder peeked); no
	// separate tag byte here. Streaming-prefix mode (own-field):
	// carrier is part of the prefix fields and emits via the same
	// per-field path.
	r := make([]byte, 0, 3)
	// Append the active arm body's encoded bytes.
	switch {
	case s.Body.CodecDefaultMarkerArmA != nil:
		r = append(r, s.Body.CodecDefaultMarkerArmA.Encode()...)
	case s.Body.CodecDefaultMarkerArmB != nil:
		r = append(r, s.Body.CodecDefaultMarkerArmB.Encode()...)
	case s.Body.Default != nil:
		r = append(r, s.Body.Default.Body.Encode()...)
	}
	return r
}