// SCE-MAP: codec_subbyte:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_subbyte

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecSubbyte represents the codec frame layout.
type CodecSubbyte struct {
	Priority uint8
	Channel uint8
	Direction uint8
}

// DecodeCodecSubbyte decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecSubbyte(cursor *codec.SceCursor) (*CodecSubbyte, error) {
	raw, err := cursor.PeekSlice(1)
	if err != nil {
		return nil, err
	}
	value := &CodecSubbyte{
		Priority: (raw[0] >> 5) & 0x07,
		Channel: (raw[0] >> 2) & 0x07,
		Direction: (raw[0] >> 0) & 0x03,
	}
	if err := cursor.Advance(1); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode serializes the CodecSubbyte into raw bytes.
func (s *CodecSubbyte) Encode() []byte {
	return []byte{
		byte((s.Priority & 0x07) << 5 | (s.Channel & 0x07) << 2 | (s.Direction & 0x03) << 0),
	}
}
