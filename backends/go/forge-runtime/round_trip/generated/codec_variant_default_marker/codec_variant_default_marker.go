// SCE-GENERATED — DO NOT EDIT
// source-hash: 6a06e9c790d7c99ded460925699421079f0734d06570cc7b08b90568979799bd
// template-hash: 08524b6e9f06ec235417da53ac7c80c6bfd4ac29c2f21bcfec9a9e720a464526
// generated-at: 0
// SCE-MAP: codec_variant_default_marker.scxml:30 :: _forge_body

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
// observed tag back onto the wire (RFC §synth-5-B variant primitive).
type CodecVariantDefaultMarkerDefault struct {
	Tag uint8
	Body codec_default_marker_arm_b.CodecDefaultMarkerArmB
}

// CodecVariantDefaultMarkerVariant is a discriminated-union body for the codec's
// tag-field suffix (RFC §synth-5-B variant primitive). Exactly one of
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
// RFC variant-default-uniformity (Go).
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
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecVariantDefaultMarker(cursor *codec.SceCursor) (*CodecVariantDefaultMarker, error) {
	// RFC §synth-5-B peek-byte / streaming-prefix:
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

// Encode writes the CodecVariantDefaultMarker into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecVariantDefaultMarker) Encode(w codec.SceSink) error {
	// RFC §synth-5-B peek-byte / streaming-prefix.
	// Append the active arm body's encoded bytes via the same sink.
	switch {
	case s.Body.CodecDefaultMarkerArmA != nil:
		if err := s.Body.CodecDefaultMarkerArmA.Encode(w); err != nil {
			return err
		}
	case s.Body.CodecDefaultMarkerArmB != nil:
		if err := s.Body.CodecDefaultMarkerArmB.Encode(w); err != nil {
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
func (s *CodecVariantDefaultMarker) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 3)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
