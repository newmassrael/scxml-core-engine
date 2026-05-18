// SCE-GENERATED — DO NOT EDIT
// source-hash: 9dba19024112d81b0a74dd568083d98ea9f9b26f403a3af95dfa629b72fd2464
// template-hash: 424c6f33953fe7f160097f9838fbc995a6528c2c939f5d44e179aa010bc5eec7
// generated-at: 1779070329
// SCE-MAP: codec_default_marker_arm_b.scxml:14

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_default_marker_arm_b

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecDefaultMarkerArmB represents the codec frame layout.
type CodecDefaultMarkerArmB struct {
	Header uint8
	Payload uint16
}

// NewCodecDefaultMarkerArmB returns a CodecDefaultMarkerArmB initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecDefaultMarkerArmB().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecDefaultMarkerArmB{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity Atomic β-go.
func NewCodecDefaultMarkerArmB() *CodecDefaultMarkerArmB {
	return &CodecDefaultMarkerArmB{
		Header: uint8(0x02),
	}
}

// DecodeCodecDefaultMarkerArmB decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecDefaultMarkerArmB(cursor *codec.SceCursor) (*CodecDefaultMarkerArmB, error) {
	raw, err := cursor.PeekSlice(3)
	if err != nil {
		return nil, err
	}
	value := &CodecDefaultMarkerArmB{
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
func (s *CodecDefaultMarkerArmB) Kind() uint8 {
	return uint8((s.Header >> 0) & 0x03)
}

func (s *CodecDefaultMarkerArmB) SetKind(v uint8) {
	const _shiftedMask uint8 = 0x03 << 0
	_val := (uint8(v) & 0x03) << 0
	s.Header = (s.Header &^ _shiftedMask) | _val
}

// Encode serializes the CodecDefaultMarkerArmB into raw bytes.
func (s *CodecDefaultMarkerArmB) Encode() []byte {
	return []byte{
		byte(s.Header),
		byte(s.Payload >> 8 & 0xFF),
		byte(s.Payload & 0xFF),
	}
}