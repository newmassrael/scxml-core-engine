// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_scout_zid_body

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecScoutZidBody represents the codec frame layout.
type CodecScoutZidBody struct {
	ZidLenM1 uint8
	Zid []byte
}

// DecodeCodecScoutZidBody decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecScoutZidBody(cursor *codec.SceCursor) (*CodecScoutZidBody, error) {
	frameLen := cursor.Remaining()
	if frameLen < 1 {
		return nil, codec.ErrNeedMoreBytes
	}
	raw, err := cursor.PeekSlice(frameLen)
	if err != nil {
		return nil, err
	}
	value := &CodecScoutZidBody{
		ZidLenM1: raw[0],
		Zid: raw[1:1+int(raw[0]) + 1],
	}
	if err := cursor.Advance(frameLen); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode serializes the CodecScoutZidBody into raw bytes.
func (s *CodecScoutZidBody) Encode() []byte {
	r := make([]byte, 0, 17)
	r = append(r, byte(s.ZidLenM1))
	r = append(r, s.Zid...)
	return r
}
