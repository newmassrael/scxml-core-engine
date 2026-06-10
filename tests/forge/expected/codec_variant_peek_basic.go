// SCE-MAP: codec_variant_peek_basic:29

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_variant_peek_basic

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_peek_arm_a"
	"example.com/sce-forge/codec_peek_arm_b"
)

// CodecVariantPeekBasicVariant is a discriminated-union body for the codec's
// tag-field suffix (RFC §synth-5-B variant primitive). Exactly one of
// the pointer fields is non-nil at a time; the active arm is the one
// that matches the current tag value.
type CodecVariantPeekBasicVariant struct {
	CodecPeekArmA *codec_peek_arm_a.CodecPeekArmA
	CodecPeekArmB *codec_peek_arm_b.CodecPeekArmB
}

// CodecVariantPeekBasic represents the codec frame layout.
type CodecVariantPeekBasic struct {
	Body CodecVariantPeekBasicVariant
}

// NewCodecVariantPeekBasic returns a CodecVariantPeekBasic initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecVariantPeekBasic().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecVariantPeekBasic{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecVariantPeekBasic() *CodecVariantPeekBasic {
	return &CodecVariantPeekBasic{
		Body: CodecVariantPeekBasicVariant{
			CodecPeekArmA: codec_peek_arm_a.NewCodecPeekArmA(),
		},
	}
}

// DecodeCodecVariantPeekBasic decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecVariantPeekBasic(cursor *codec.SceCursor) (*CodecVariantPeekBasic, error) {
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
	body := CodecVariantPeekBasicVariant{}
	switch uint8((_peek >> 0) & 0x01) {
	case 0:
		_arm, err := codec_peek_arm_a.DecodeCodecPeekArmA(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecPeekArmA = _arm
	case 1:
		_arm, err := codec_peek_arm_b.DecodeCodecPeekArmB(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecPeekArmB = _arm
	default:
		// codec/variant-arm-unreachable rejected this case at parse time.
		return nil, codec.ErrNeedMoreBytes
	}
	return &CodecVariantPeekBasic{
		Body: body,
	}, nil
}

// Encode writes the CodecVariantPeekBasic into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecVariantPeekBasic) Encode(w codec.SceSink) error {
	// RFC §synth-5-B peek-byte / streaming-prefix.
	// Append the active arm body's encoded bytes via the same sink.
	switch {
	case s.Body.CodecPeekArmA != nil:
		if err := s.Body.CodecPeekArmA.Encode(w); err != nil {
			return err
		}
	case s.Body.CodecPeekArmB != nil:
		if err := s.Body.CodecPeekArmB.Encode(w); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecVariantPeekBasic) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 3)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
