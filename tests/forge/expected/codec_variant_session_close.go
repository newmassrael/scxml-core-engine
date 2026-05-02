// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_variant_session_close

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecVariantSessionClose represents the codec frame layout.
type CodecVariantSessionClose struct {
	Reason uint8
}

// DecodeCodecVariantSessionClose decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecVariantSessionClose(cursor *codec.SceCursor) (*CodecVariantSessionClose, error) {
	raw, err := cursor.PeekSlice(1)
	if err != nil {
		return nil, err
	}
	value := &CodecVariantSessionClose{
		Reason: raw[0],
	}
	if err := cursor.Advance(1); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode serializes the CodecVariantSessionClose into raw bytes.
func (s *CodecVariantSessionClose) Encode() []byte {
	return []byte{
		byte(s.Reason),
	}
}
