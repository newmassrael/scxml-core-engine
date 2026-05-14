// SCE-MAP: codec_variant_session_open:5

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_variant_session_open

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecVariantSessionOpen represents the codec frame layout.
type CodecVariantSessionOpen struct {
	Version uint16
}

// DecodeCodecVariantSessionOpen decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecVariantSessionOpen(cursor *codec.SceCursor) (*CodecVariantSessionOpen, error) {
	raw, err := cursor.PeekSlice(2)
	if err != nil {
		return nil, err
	}
	value := &CodecVariantSessionOpen{
		Version: uint16(raw[0])<<8 | uint16(raw[1]),
	}
	if err := cursor.Advance(2); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode serializes the CodecVariantSessionOpen into raw bytes.
func (s *CodecVariantSessionOpen) Encode() []byte {
	return []byte{
		byte(s.Version >> 8 & 0xFF),
		byte(s.Version & 0xFF),
	}
}
