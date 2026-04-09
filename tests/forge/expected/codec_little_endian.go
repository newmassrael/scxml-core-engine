// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Do not edit — regenerate from the source SCXML file.

package codec_little_endian

// CodecLittleEndian represents the codec frame layout.
type CodecLittleEndian struct {
	SensorId uint8
	Value uint16
	Status uint8
}

// DecodeCodecLittleEndian decodes raw bytes into a CodecLittleEndian.
// Returns nil if the input is too short.
func DecodeCodecLittleEndian(raw []byte) *CodecLittleEndian {
	if len(raw) < 4 {
		return nil
	}
	return &CodecLittleEndian{
		SensorId: raw[0],
		Value: uint16(raw[1]) | uint16(raw[2])<<8,
		Status: raw[3],
	}
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