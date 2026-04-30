// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_length_ref

// CodecLengthRef represents the codec frame layout.
type CodecLengthRef struct {
	MsgId uint8
	Len uint8
	Payload []byte
}

// DecodeCodecLengthRef decodes raw bytes into a CodecLengthRef.
// Returns nil if the input is too short.
func DecodeCodecLengthRef(raw []byte) *CodecLengthRef {
	if len(raw) < 2 {
		return nil
	}
	return &CodecLengthRef{
		MsgId: raw[0],
		Len: raw[1],
		Payload: raw[2:2+int(raw[1])],
	}
}

// Encode serializes the CodecLengthRef into raw bytes.
func (s *CodecLengthRef) Encode() []byte {
	r := make([]byte, 0, 34)
	r = append(r, byte(s.MsgId))
	r = append(r, byte(s.Len))
	r = append(r, s.Payload...)
	return r
}
