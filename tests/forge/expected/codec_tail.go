// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_tail

// CodecTail represents the codec frame layout.
type CodecTail struct {
	MsgId uint8
	Status uint8
	Payload []byte
}

// DecodeCodecTail decodes raw bytes into a CodecTail.
// Returns nil if the input is too short.
func DecodeCodecTail(raw []byte) *CodecTail {
	if len(raw) < 2 {
		return nil
	}
	return &CodecTail{
		MsgId: raw[0],
		Status: raw[1],
		Payload: raw[2:],
	}
}

// Encode serializes the CodecTail into raw bytes.
func (s *CodecTail) Encode() []byte {
	r := make([]byte, 0, 34)
	r = append(r, byte(s.MsgId))
	r = append(r, byte(s.Status))
	r = append(r, s.Payload...)
	return r
}
