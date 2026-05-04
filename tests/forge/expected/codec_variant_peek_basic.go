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
// tag-field suffix (RFC §5.B variant primitive B1-β). Exactly one of
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

// DecodeCodecVariantPeekBasic decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecVariantPeekBasic(cursor *codec.SceCursor) (*CodecVariantPeekBasic, error) {
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

// Encode serializes the CodecVariantPeekBasic into raw bytes.
func (s *CodecVariantPeekBasic) Encode() []byte {
	// RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
	// streaming prefix encode. Peek-byte mode: arm body's encode
	// prepends its own header byte (which the decoder peeked); no
	// separate tag byte here. Streaming-prefix mode (own-field):
	// carrier is part of the prefix fields and emits via the same
	// per-field path.
	r := make([]byte, 0, 3)
	// Append the active arm body's encoded bytes.
	switch {
	case s.Body.CodecPeekArmA != nil:
		r = append(r, s.Body.CodecPeekArmA.Encode()...)
	case s.Body.CodecPeekArmB != nil:
		r = append(r, s.Body.CodecPeekArmB.Encode()...)
	}
	return r
}
