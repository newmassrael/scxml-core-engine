// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_embed_basic

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_locator"
)

// CodecEmbedBasic represents the codec frame layout.
type CodecEmbedBasic struct {
	Tag uint8
	Locator codec_zenoh_locator.CodecZenohLocator
}

// DecodeCodecEmbedBasic decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecEmbedBasic(cursor *codec.SceCursor) (*CodecEmbedBasic, error) {
	// RFC §5.B B2 repeat primitive: streaming decode mixes plain
	// fixed-width reads (per-field via the present-if helper's
	// non-gated arm) with `make([]T, 0, N)` + `append` loops that
	// iterate the imported codec's `Decode<T>(cursor)` either
	// `int(N)` times (length-field) or until `cursor.Remaining() == 0`
	// (until-eof). Element bodies recurse into their own decoder —
	// each may itself surface `codec.ErrNeedMoreBytes`, unwinding
	// the partial frame.
	var Tag uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Tag = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	Locator, err := codec_zenoh_locator.DecodeCodecZenohLocator(cursor)
	if err != nil {
		return nil, err
	}
	return &CodecEmbedBasic{
		Tag: Tag,
		Locator: Locator,
	}, nil
}

// Encode serializes the CodecEmbedBasic into raw bytes.
func (s *CodecEmbedBasic) Encode() []byte {
	// RFC §5.B B2 encode: fixed prefix appends byte-by-byte; repeat
	// fields range over `s.<Pascal>` and spread each element's
	// `Encode()` bytes into the parent buffer. Author keeps count
	// field == slice length (trust contract).
	r := make([]byte, 0, 257)
	r = append(r, s.Tag)
	r = append(r, s.Locator.Encode()...)
	return r
}
