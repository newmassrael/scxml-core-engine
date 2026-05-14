// SCE-MAP: codec_zenoh_keep_alive:10

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_keep_alive

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohKeepAlive represents the codec frame layout.
type CodecZenohKeepAlive struct {
}

// DecodeCodecZenohKeepAlive decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohKeepAlive(cursor *codec.SceCursor) (*CodecZenohKeepAlive, error) {
	// RFC §5.B B5-α empty body — zero-byte payload, no cursor work.
	_ = cursor
	return &CodecZenohKeepAlive{}, nil
}

// Encode serializes the CodecZenohKeepAlive into raw bytes.
func (s *CodecZenohKeepAlive) Encode() []byte {
	// RFC §5.B B5-α empty body — zero-byte payload.
	return []byte{}
}
