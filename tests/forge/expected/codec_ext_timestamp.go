// SCE-MAP: codec_ext_timestamp:24

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_ext_timestamp

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecExtTimestamp represents the codec frame layout.
type CodecExtTimestamp struct {
	Time uint64
	ZidSize uint8
	Zid []byte
}

// DecodeCodecExtTimestamp decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecExtTimestamp(cursor *codec.SceCursor) (*CodecExtTimestamp, error) {
	// Streaming codec: each field reads from cursor directly
	// (VLE base-128 chain). Local var name reuses the Go-PascalCase
	// `field.id` — the struct literal's `Foo: Foo` is unambiguous
	// because the package owns both names. RFC §5.B B4: per-field
	// bit-size dispatch routes Fixed / LengthRef siblings of VLE
	// fields through `present_if_decode_stmt` (predicate=None arms).
	// Pure-VLE codecs stay byte-stable.
	Time, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var ZidSize uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		ZidSize = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var Zid []byte
	{
		_n := int(ZidSize)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Zid = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	return &CodecExtTimestamp{
		Time: Time,
		ZidSize: ZidSize,
		Zid: Zid,
	}, nil
}

// Encode serializes the CodecExtTimestamp into raw bytes.
func (s *CodecExtTimestamp) Encode() []byte {
	// RFC §5.B B4: per-field bit-size dispatch routes Fixed /
	// LengthRef siblings of VLE fields through
	// `present_if_encode_block` (predicate=None arms). Pure-VLE
	// codecs stay byte-stable.
	r := make([]byte, 0, 28)
	{
		_w := uint64(s.Time)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	r = append(r, s.ZidSize)
	r = append(r, s.Zid...)
	return r
}
