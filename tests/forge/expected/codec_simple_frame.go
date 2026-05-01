// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_simple_frame

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecSimpleFrame represents the codec frame layout.
type CodecSimpleFrame struct {
	MsgId uint8
	Length uint8
	Payload uint16
}

// DecodeCodecSimpleFrame decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecSimpleFrame(cursor *codec.SceCursor) (*CodecSimpleFrame, error) {
	raw, err := cursor.PeekSlice(4)
	if err != nil {
		return nil, err
	}
	value := &CodecSimpleFrame{
		MsgId: raw[0],
		Length: raw[1],
		Payload: uint16(raw[2])<<8 | uint16(raw[3]),
	}
	if err := cursor.Advance(4); err != nil {
		return nil, err
	}
	return value, nil
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
