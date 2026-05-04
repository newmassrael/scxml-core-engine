// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_timestamp

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohTimestamp represents the codec frame layout.
type CodecZenohTimestamp struct {
	Time uint64
	ZidLen uint64
	Zid []byte
}

// DecodeCodecZenohTimestamp decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohTimestamp(cursor *codec.SceCursor) (*CodecZenohTimestamp, error) {
	// Streaming codec: each field reads from cursor directly
	// (VLE base-128 chain). Local var name reuses the Go-PascalCase
	// `field.id` — the struct literal's `Foo: Foo` is unambiguous
	// because the package owns both names. RFC §5.B B4: per-field
	// bit-size dispatch routes Fixed / LengthRef siblings of VLE
	// fields through `present_if_decode_stmt` (predicate=None arms).
	// Pure-VLE codecs stay byte-stable.
	Time, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	ZidLen, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var Zid []byte
	{
		_n := int(ZidLen)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Zid = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	return &CodecZenohTimestamp{
		Time: Time,
		ZidLen: ZidLen,
		Zid: Zid,
	}, nil
}

// Encode serializes the CodecZenohTimestamp into raw bytes.
func (s *CodecZenohTimestamp) Encode() []byte {
	// RFC §5.B B4: per-field bit-size dispatch routes Fixed /
	// LengthRef siblings of VLE fields through
	// `present_if_encode_block` (predicate=None arms). Pure-VLE
	// codecs stay byte-stable.
	r := make([]byte, 0, 36)
	{
		_w := uint64(s.Time)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	{
		_w := uint64(s.ZidLen)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	r = append(r, s.Zid...)
	return r
}
