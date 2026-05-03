// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_del

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohDel represents the codec frame layout.
type CodecZenohDel struct {
}

// DecodeCodecZenohDel decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohDel(cursor *codec.SceCursor) (*CodecZenohDel, error) {
	// RFC §5.B B5-α empty body — zero-byte payload, no cursor work.
	_ = cursor
	return &CodecZenohDel{}, nil
}

// Encode serializes the CodecZenohDel into raw bytes.
func (s *CodecZenohDel) Encode() []byte {
	// RFC §5.B B5-α empty body — zero-byte payload.
	return []byte{}
}
