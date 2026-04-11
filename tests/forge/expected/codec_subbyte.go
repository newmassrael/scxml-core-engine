// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Do not edit — regenerate from the source SCXML file.

package codec_subbyte

// CodecSubbyte represents the codec frame layout.
type CodecSubbyte struct {
	Priority uint8
	Channel uint8
	Direction uint8
}

// DecodeCodecSubbyte decodes raw bytes into a CodecSubbyte.
// Returns nil if the input is too short.
func DecodeCodecSubbyte(raw []byte) *CodecSubbyte {
	if len(raw) < 1 {
		return nil
	}
	return &CodecSubbyte{
		Priority: (raw[0] >> 5) & 0x07,
		Channel: (raw[0] >> 2) & 0x07,
		Direction: (raw[0] >> 0) & 0x03,
	}
}

// Encode serializes the CodecSubbyte into raw bytes.
func (s *CodecSubbyte) Encode() []byte {
	return []byte{
		byte((s.Priority & 0x07) << 5 | (s.Channel & 0x07) << 2 | (s.Direction & 0x03) << 0),
	}
}
