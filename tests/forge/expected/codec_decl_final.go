// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_decl_final

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecDeclFinal represents the codec frame layout.
type CodecDeclFinal struct {
}

// DecodeCodecDeclFinal decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecDeclFinal(cursor *codec.SceCursor) (*CodecDeclFinal, error) {
	// RFC §5.B B5-α empty body — zero-byte payload, no cursor work.
	_ = cursor
	return &CodecDeclFinal{}, nil
}

// Encode serializes the CodecDeclFinal into raw bytes.
func (s *CodecDeclFinal) Encode() []byte {
	// RFC §5.B B5-α empty body — zero-byte payload.
	return []byte{}
}
