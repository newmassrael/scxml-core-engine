// SCE-MAP: codec_zenoh_decl_final:19

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_decl_final

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohDeclFinal represents the codec frame layout.
type CodecZenohDeclFinal struct {
}

// DecodeCodecZenohDeclFinal decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohDeclFinal(cursor *codec.SceCursor) (*CodecZenohDeclFinal, error) {
	// RFC §5.B B5-α empty body — zero-byte payload, no cursor work.
	_ = cursor
	return &CodecZenohDeclFinal{}, nil
}

// Encode serializes the CodecZenohDeclFinal into raw bytes.
func (s *CodecZenohDeclFinal) Encode() []byte {
	// RFC §5.B B5-α empty body — zero-byte payload.
	return []byte{}
}
