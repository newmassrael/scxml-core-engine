// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_timestamp_ext

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_timestamp"
)

// CodecZenohTimestampExt represents the codec frame layout.
type CodecZenohTimestampExt struct {
	ExtSize uint64
	Ts codec_zenoh_timestamp.CodecZenohTimestamp
}

// DecodeCodecZenohTimestampExt decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohTimestampExt(cursor *codec.SceCursor) (*CodecZenohTimestampExt, error) {
	// Streaming codec: each field reads from cursor directly
	// (VLE base-128 chain). Local var name reuses the Go-PascalCase
	// `field.id` — the struct literal's `Foo: Foo` is unambiguous
	// because the package owns both names. RFC §5.B B4: per-field
	// bit-size dispatch routes Fixed / LengthRef siblings of VLE
	// fields through `present_if_decode_stmt` (predicate=None arms).
	// Pure-VLE codecs stay byte-stable.
	ExtSize, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var Ts codec_zenoh_timestamp.CodecZenohTimestamp
	{
		_len := int(ExtSize)
		_raw, err := cursor.PeekSlice(_len)
		if err != nil {
			return nil, err
		}
		_inner := codec.NewSceCursor(_raw)
		_emb, err := codec_zenoh_timestamp.DecodeCodecZenohTimestamp(&_inner)
		if err != nil {
			return nil, err
		}
		if err := cursor.Advance(_len); err != nil {
			return nil, err
		}
		Ts = *_emb
	}
	return &CodecZenohTimestampExt{
		ExtSize: ExtSize,
		Ts: Ts,
	}, nil
}

// Encode serializes the CodecZenohTimestampExt into raw bytes.
func (s *CodecZenohTimestampExt) Encode() []byte {
	// RFC §5.B B4: per-field bit-size dispatch routes Fixed /
	// LengthRef siblings of VLE fields through
	// `present_if_encode_block` (predicate=None arms). Pure-VLE
	// codecs stay byte-stable.
	r := make([]byte, 0, 266)
	{
		_w := uint64(s.ExtSize)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	r = append(r, s.Ts.Encode()...)
	return r
}
