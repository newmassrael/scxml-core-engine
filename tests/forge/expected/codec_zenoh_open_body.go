// SCE-MAP: codec_zenoh_open_body:41

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_open_body

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohOpenBody represents the codec frame layout.
type CodecZenohOpenBody struct {
	Lease uint64
	InitialSn uint64
	CookieLen *uint64
	Cookie []byte
}

// DecodeCodecZenohOpenBody decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohOpenBody(cursor *codec.SceCursor, parentFlags byte) (*CodecZenohOpenBody, error) {
	// RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
	// advances the cursor per field. Gated fields use `*T` for fixed
	// (nil = absent) or `[]byte` (nil = absent) for tail/length-ref;
	// VLE gating uses `*T` like fixed. Per-field `is_repeat` routes
	// Repeat fields to the dedicated helper. Branch fires before
	// has_vle_fields so a codec mixing VLE + present-if uses the
	// unified streaming path.
	Lease, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	InitialSn, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var CookieLen *uint64
	if (parentFlags & 0x20) == 0 {
		_v, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
		CookieLen = &_v
	}
	var Cookie []byte
	if (parentFlags & 0x20) == 0 {
		_n := int(*CookieLen)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Cookie = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	return &CodecZenohOpenBody{
		Lease: Lease,
		InitialSn: InitialSn,
		CookieLen: CookieLen,
		Cookie: Cookie,
	}, nil
}

// Encode serializes the CodecZenohOpenBody into raw bytes.
func (s *CodecZenohOpenBody) Encode(parentFlags byte) []byte {
	// RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
	// Gated fields skip the append on nil pointer / nil slice. Per-
	// field `is_repeat` routes Repeat fields to the dedicated helper.
	// Branch fires before has_vle_fields so a codec mixing VLE +
	// present-if uses the unified encode path.
	r := make([]byte, 0, 158)
	{
		_w := uint64(s.Lease)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	{
		_w := uint64(s.InitialSn)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	if s.CookieLen != nil {
		_v := *s.CookieLen
	{
		_w := uint64(_v)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	}
	if s.Cookie != nil {
		r = append(r, s.Cookie...)
	}
	return r
}
