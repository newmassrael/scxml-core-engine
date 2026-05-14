// SCE-MAP: codec_ext_attachment:27

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_ext_attachment

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecExtAttachment represents the codec frame layout.
type CodecExtAttachment struct {
	Length uint8
	Body []byte
}

// DecodeCodecExtAttachment decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecExtAttachment(cursor *codec.SceCursor) (*CodecExtAttachment, error) {
	frameLen := cursor.Remaining()
	if frameLen < 1 {
		return nil, codec.ErrNeedMoreBytes
	}
	raw, err := cursor.PeekSlice(frameLen)
	if err != nil {
		return nil, err
	}
	value := &CodecExtAttachment{
		Length: raw[0],
		Body: raw[1:1+int(raw[0])],
	}
	if err := cursor.Advance(frameLen); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode serializes the CodecExtAttachment into raw bytes.
func (s *CodecExtAttachment) Encode() []byte {
	r := make([]byte, 0, 65)
	r = append(r, byte(s.Length))
	r = append(r, s.Body...)
	return r
}
