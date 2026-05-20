// SCE-MAP: codec_tail:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_tail

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecTail represents the codec frame layout.
type CodecTail struct {
	MsgId uint8
	Status uint8
	Payload []byte
}

// DecodeCodecTail decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecTail(cursor *codec.SceCursor) (*CodecTail, error) {
	frameLen := cursor.Remaining()
	if frameLen < 2 {
		return nil, codec.ErrNeedMoreBytes
	}
	raw, err := cursor.PeekSlice(frameLen)
	if err != nil {
		return nil, err
	}
	MsgId := raw[0]
	Status := raw[1]
	Payload := raw[2:]
	value := &CodecTail{
		MsgId: MsgId,
		Status: Status,
		Payload: Payload,
	}
	if err := cursor.Advance(frameLen); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode serializes the CodecTail into raw bytes.
func (s *CodecTail) Encode() []byte {
	r := make([]byte, 0, 34)
	r = append(r, byte(s.MsgId))
	r = append(r, byte(s.Status))
	r = append(r, s.Payload...)
	return r
}
