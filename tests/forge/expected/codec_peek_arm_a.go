// SCE-MAP: codec_peek_arm_a:13

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_peek_arm_a

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecPeekArmA represents the codec frame layout.
type CodecPeekArmA struct {
	Header uint8
	Payload uint8
}

// DecodeCodecPeekArmA decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecPeekArmA(cursor *codec.SceCursor) (*CodecPeekArmA, error) {
	raw, err := cursor.PeekSlice(2)
	if err != nil {
		return nil, err
	}
	value := &CodecPeekArmA{
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
func (s *CodecPeekArmA) Kind() bool {
	return (s.Header & 0x01) != 0
}

func (s *CodecPeekArmA) SetKind(v bool) {
	if v {
		s.Header |= 0x01
	} else {
		s.Header &^= 0x01
	}
}

// Encode serializes the CodecPeekArmA into raw bytes.
func (s *CodecPeekArmA) Encode() []byte {
	return []byte{
		byte(s.Header),
		byte(s.Payload),
	}
}
