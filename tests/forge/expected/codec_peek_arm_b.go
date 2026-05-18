// SCE-MAP: codec_peek_arm_b:13

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_peek_arm_b

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecPeekArmB represents the codec frame layout.
type CodecPeekArmB struct {
	Header uint8
	Payload uint16
}

// NewCodecPeekArmB returns a CodecPeekArmB initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecPeekArmB().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecPeekArmB{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity Atomic β-go.
func NewCodecPeekArmB() *CodecPeekArmB {
	return &CodecPeekArmB{
		Header: uint8(0x01),
	}
}

// DecodeCodecPeekArmB decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecPeekArmB(cursor *codec.SceCursor) (*CodecPeekArmB, error) {
	raw, err := cursor.PeekSlice(3)
	if err != nil {
		return nil, err
	}
	value := &CodecPeekArmB{
		Header: raw[0],
		Payload: uint16(raw[1])<<8 | uint16(raw[2]),
	}
	if err := cursor.Advance(3); err != nil {
		return nil, err
	}
	return value, nil
}

// RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecPeekArmB) Kind() bool {
	return (s.Header & 0x01) != 0
}

func (s *CodecPeekArmB) SetKind(v bool) {
	if v {
		s.Header |= 0x01
	} else {
		s.Header &^= 0x01
	}
}

// Encode serializes the CodecPeekArmB into raw bytes.
func (s *CodecPeekArmB) Encode() []byte {
	return []byte{
		byte(s.Header),
		byte(s.Payload >> 8 & 0xFF),
		byte(s.Payload & 0xFF),
	}
}
