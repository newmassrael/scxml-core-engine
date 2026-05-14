// SCE-MAP: codec_zenoh_ext_zint:11

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_ext_zint

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohExtZint represents the codec frame layout.
type CodecZenohExtZint struct {
	Value uint64
}

// DecodeCodecZenohExtZint decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohExtZint(cursor *codec.SceCursor) (*CodecZenohExtZint, error) {
	// Streaming codec: each field reads from cursor directly
	// (VLE base-128 chain). Local var name reuses the Go-PascalCase
	// `field.id` — the struct literal's `Foo: Foo` is unambiguous
	// because the package owns both names. RFC §5.B B4: per-field
	// bit-size dispatch routes Fixed / LengthRef siblings of VLE
	// fields through `present_if_decode_stmt` (predicate=None arms).
	// Pure-VLE codecs stay byte-stable.
	Value, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	return &CodecZenohExtZint{
		Value: Value,
	}, nil
}

// Encode serializes the CodecZenohExtZint into raw bytes.
func (s *CodecZenohExtZint) Encode() []byte {
	// RFC §5.B B4: per-field bit-size dispatch routes Fixed /
	// LengthRef siblings of VLE fields through
	// `present_if_encode_block` (predicate=None arms). Pure-VLE
	// codecs stay byte-stable.
	r := make([]byte, 0, 10)
	{
		_w := uint64(s.Value)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	return r
}
