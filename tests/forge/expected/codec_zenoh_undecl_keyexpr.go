// SCE-MAP: codec_zenoh_undecl_keyexpr:35

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_undecl_keyexpr

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohUndeclKeyexpr represents the codec frame layout.
type CodecZenohUndeclKeyexpr struct {
	Id uint16
}

// DecodeCodecZenohUndeclKeyexpr decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohUndeclKeyexpr(cursor *codec.SceCursor) (*CodecZenohUndeclKeyexpr, error) {
	// Streaming codec: each field reads from cursor directly
	// (VLE base-128 chain). Local var name reuses the Go-PascalCase
	// `field.id` — the struct literal's `Foo: Foo` is unambiguous
	// because the package owns both names. RFC §5.B B4: per-field
	// bit-size dispatch routes Fixed / LengthRef siblings of VLE
	// fields through `present_if_decode_stmt` (predicate=None arms).
	// Pure-VLE codecs stay byte-stable.
	Id, err := cursor.ReadVLEU16()
	if err != nil { return nil, err }
	return &CodecZenohUndeclKeyexpr{
		Id: Id,
	}, nil
}

// Encode serializes the CodecZenohUndeclKeyexpr into raw bytes.
func (s *CodecZenohUndeclKeyexpr) Encode() []byte {
	// RFC §5.B B4: per-field bit-size dispatch routes Fixed /
	// LengthRef siblings of VLE fields through
	// `present_if_encode_block` (predicate=None arms). Pure-VLE
	// codecs stay byte-stable.
	r := make([]byte, 0, 3)
	{
		_w := uint64(s.Id)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	return r
}
