// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_little_endian

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecLittleEndian represents the codec frame layout.
type CodecLittleEndian struct {
	SensorId uint8
	Value uint16
	Status uint8
}

// DecodeCodecLittleEndian decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
func DecodeCodecLittleEndian(cursor *codec.SceCursor) (*CodecLittleEndian, error) {
	raw, err := cursor.PeekSlice(4)
	if err != nil {
		return nil, err
	}
	value := &CodecLittleEndian{
		SensorId: raw[0],
		Value: uint16(raw[1]) | uint16(raw[2])<<8,
		Status: raw[3],
	}
	if err := cursor.Advance(4); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode serializes the CodecLittleEndian into raw bytes.
func (s *CodecLittleEndian) Encode() []byte {
	return []byte{
		byte(s.SensorId),
		byte(s.Value & 0xFF),
		byte(s.Value >> 8 & 0xFF),
		byte(s.Status),
	}
}
