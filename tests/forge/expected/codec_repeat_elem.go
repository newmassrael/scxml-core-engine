// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_repeat_elem

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecRepeatElem represents the codec frame layout.
type CodecRepeatElem struct {
	Seq uint16
}

// DecodeCodecRepeatElem decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecRepeatElem(cursor *codec.SceCursor) (*CodecRepeatElem, error) {
	raw, err := cursor.PeekSlice(2)
	if err != nil {
		return nil, err
	}
	value := &CodecRepeatElem{
		Seq: uint16(raw[0])<<8 | uint16(raw[1]),
	}
	if err := cursor.Advance(2); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode serializes the CodecRepeatElem into raw bytes.
func (s *CodecRepeatElem) Encode() []byte {
	return []byte{
		byte(s.Seq >> 8 & 0xFF),
		byte(s.Seq & 0xFF),
	}
}
