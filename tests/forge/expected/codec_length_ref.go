// SCE-MAP: codec_length_ref:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_length_ref

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecLengthRef represents the codec frame layout.
type CodecLengthRef struct {
	MsgId uint8
	Len uint8
	Payload []byte
}

// DecodeCodecLengthRef decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecLengthRef(cursor *codec.SceCursor) (*CodecLengthRef, error) {
	frameLen := cursor.Remaining()
	if frameLen < 2 {
		return nil, codec.ErrNeedMoreBytes
	}
	raw, err := cursor.PeekSlice(frameLen)
	if err != nil {
		return nil, err
	}
	value := &CodecLengthRef{
		MsgId: raw[0],
		Len: raw[1],
		Payload: raw[2:2+int(raw[1])],
	}
	if err := cursor.Advance(frameLen); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode serializes the CodecLengthRef into raw bytes.
func (s *CodecLengthRef) Encode() []byte {
	r := make([]byte, 0, 34)
	r = append(r, byte(s.MsgId))
	r = append(r, byte(s.Len))
	r = append(r, s.Payload...)
	return r
}
