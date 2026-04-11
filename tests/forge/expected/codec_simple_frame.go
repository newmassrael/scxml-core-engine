// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Do not edit — regenerate from the source SCXML file.

package codec_simple_frame

// CodecSimpleFrame represents the codec frame layout.
type CodecSimpleFrame struct {
	MsgId uint8
	Length uint8
	Payload uint16
}

// DecodeCodecSimpleFrame decodes raw bytes into a CodecSimpleFrame.
// Returns nil if the input is too short.
func DecodeCodecSimpleFrame(raw []byte) *CodecSimpleFrame {
	if len(raw) < 4 {
		return nil
	}
	return &CodecSimpleFrame{
		MsgId: raw[0],
		Length: raw[1],
		Payload: uint16(raw[2])<<8 | uint16(raw[3]),
	}
}

// Encode serializes the CodecSimpleFrame into raw bytes.
func (s *CodecSimpleFrame) Encode() []byte {
	return []byte{
		byte(s.MsgId),
		byte(s.Length),
		byte(s.Payload >> 8 & 0xFF),
		byte(s.Payload & 0xFF),
	}
}
