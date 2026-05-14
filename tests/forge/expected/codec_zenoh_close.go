// SCE-MAP: codec_zenoh_close:16

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_close

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohClose represents the codec frame layout.
type CodecZenohClose struct {
	Reason uint8
}

// DecodeCodecZenohClose decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohClose(cursor *codec.SceCursor) (*CodecZenohClose, error) {
	raw, err := cursor.PeekSlice(1)
	if err != nil {
		return nil, err
	}
	value := &CodecZenohClose{
		Reason: raw[0],
	}
	if err := cursor.Advance(1); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode serializes the CodecZenohClose into raw bytes.
func (s *CodecZenohClose) Encode() []byte {
	return []byte{
		byte(s.Reason),
	}
}
