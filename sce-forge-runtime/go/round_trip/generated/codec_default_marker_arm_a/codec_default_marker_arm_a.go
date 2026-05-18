// SCE-GENERATED — DO NOT EDIT
// source-hash: 9dba19024112d81b0a74dd568083d98ea9f9b26f403a3af95dfa629b72fd2464
// template-hash: 424c6f33953fe7f160097f9838fbc995a6528c2c939f5d44e179aa010bc5eec7
// generated-at: 1779070329
// SCE-MAP: codec_default_marker_arm_a.scxml:16

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_default_marker_arm_a

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecDefaultMarkerArmA represents the codec frame layout.
type CodecDefaultMarkerArmA struct {
	Header uint8
	Payload uint8
}

// NewCodecDefaultMarkerArmA returns a CodecDefaultMarkerArmA initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecDefaultMarkerArmA().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecDefaultMarkerArmA{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity Atomic β-go.
func NewCodecDefaultMarkerArmA() *CodecDefaultMarkerArmA {
	return &CodecDefaultMarkerArmA{
		Header: uint8(0x01),
	}
}

// DecodeCodecDefaultMarkerArmA decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecDefaultMarkerArmA(cursor *codec.SceCursor) (*CodecDefaultMarkerArmA, error) {
	raw, err := cursor.PeekSlice(2)
	if err != nil {
		return nil, err
	}
	value := &CodecDefaultMarkerArmA{
		Header: raw[0],
		Payload: raw[1],
	}
	if err := cursor.Advance(2); err != nil {
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
func (s *CodecDefaultMarkerArmA) Kind() uint8 {
	return uint8((s.Header >> 0) & 0x03)
}

func (s *CodecDefaultMarkerArmA) SetKind(v uint8) {
	const _shiftedMask uint8 = 0x03 << 0
	_val := (uint8(v) & 0x03) << 0
	s.Header = (s.Header &^ _shiftedMask) | _val
}

// Encode serializes the CodecDefaultMarkerArmA into raw bytes.
func (s *CodecDefaultMarkerArmA) Encode() []byte {
	return []byte{
		byte(s.Header),
		byte(s.Payload),
	}
}