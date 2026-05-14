// SCE-MAP: codec_zenoh_ext_zbuf:17

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_ext_zbuf

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohExtZbuf represents the codec frame layout.
type CodecZenohExtZbuf struct {
	ValueLen uint64
	Value []byte
}

// DecodeCodecZenohExtZbuf decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohExtZbuf(cursor *codec.SceCursor) (*CodecZenohExtZbuf, error) {
	// Streaming codec: each field reads from cursor directly
	// (VLE base-128 chain). Local var name reuses the Go-PascalCase
	// `field.id` — the struct literal's `Foo: Foo` is unambiguous
	// because the package owns both names. RFC §5.B B4: per-field
	// bit-size dispatch routes Fixed / LengthRef siblings of VLE
	// fields through `present_if_decode_stmt` (predicate=None arms).
	// Pure-VLE codecs stay byte-stable.
	ValueLen, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var Value []byte
	{
		_n := int(ValueLen)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Value = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	return &CodecZenohExtZbuf{
		ValueLen: ValueLen,
		Value: Value,
	}, nil
}

// Encode serializes the CodecZenohExtZbuf into raw bytes.
func (s *CodecZenohExtZbuf) Encode() []byte {
	// RFC §5.B B4: per-field bit-size dispatch routes Fixed /
	// LengthRef siblings of VLE fields through
	// `present_if_encode_block` (predicate=None arms). Pure-VLE
	// codecs stay byte-stable.
	r := make([]byte, 0, 42)
	{
		_w := uint64(s.ValueLen)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	r = append(r, s.Value...)
	return r
}
