// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_qos_byte

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecQosByte represents the codec frame layout.
type CodecQosByte struct {
	Qos uint8
}

// DecodeCodecQosByte decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecQosByte(cursor *codec.SceCursor) (*CodecQosByte, error) {
	raw, err := cursor.PeekSlice(1)
	if err != nil {
		return nil, err
	}
	value := &CodecQosByte{
		Qos: raw[0],
	}
	if err := cursor.Advance(1); err != nil {
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
func (s *CodecQosByte) Priority() uint8 {
	return uint8((s.Qos >> 0) & 0x07)
}

func (s *CodecQosByte) SetPriority(v uint8) {
	const _shiftedMask uint8 = 0x07 << 0
	_val := (uint8(v) & 0x07) << 0
	s.Qos = (s.Qos &^ _shiftedMask) | _val
}

func (s *CodecQosByte) Reliable() bool {
	return (s.Qos & 0x08) != 0
}

func (s *CodecQosByte) SetReliable(v bool) {
	if v {
		s.Qos |= 0x08
	} else {
		s.Qos &^= 0x08
	}
}

func (s *CodecQosByte) Congestion() uint8 {
	return uint8((s.Qos >> 4) & 0x03)
}

func (s *CodecQosByte) SetCongestion(v uint8) {
	const _shiftedMask uint8 = 0x03 << 4
	_val := (uint8(v) & 0x03) << 4
	s.Qos = (s.Qos &^ _shiftedMask) | _val
}

func (s *CodecQosByte) Express() bool {
	return (s.Qos & 0x40) != 0
}

func (s *CodecQosByte) SetExpress(v bool) {
	if v {
		s.Qos |= 0x40
	} else {
		s.Qos &^= 0x40
	}
}

func (s *CodecQosByte) Reserved() bool {
	return (s.Qos & 0x80) != 0
}

func (s *CodecQosByte) SetReserved(v bool) {
	if v {
		s.Qos |= 0x80
	} else {
		s.Qos &^= 0x80
	}
}

// Encode serializes the CodecQosByte into raw bytes.
func (s *CodecQosByte) Encode() []byte {
	return []byte{
		byte(s.Qos),
	}
}
