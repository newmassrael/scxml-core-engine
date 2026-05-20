// SCE-MAP: codec_init_cookie_body:36

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_init_cookie_body

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecInitCookieBody represents the codec frame layout.
type CodecInitCookieBody struct {
	Version uint8
	CookieSize *uint16
	Cookie []byte
}

// DecodeCodecInitCookieBody decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecInitCookieBody(cursor *codec.SceCursor, A byte) (*CodecInitCookieBody, error) {
	// RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
	// advances the cursor per field. Gated fields use `*T` for fixed
	// (nil = absent) or `[]byte` (nil = absent) for tail/length-ref;
	// VLE gating uses `*T` like fixed. Per-field `is_repeat` routes
	// Repeat fields to the dedicated helper. Branch fires before
	// has_vle_fields so a codec mixing VLE + present-if uses the
	// unified streaming path.
	var Version uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Version = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var CookieSize *uint16
	if (A & 0x01) != 0 {
		_v, err := cursor.ReadVLEU16()
	if err != nil { return nil, err }
		CookieSize = &_v
	}
	var Cookie []byte
	if (A & 0x01) != 0 {
		_n := int(*CookieSize)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Cookie = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	return &CodecInitCookieBody{
		Version: Version,
		CookieSize: CookieSize,
		Cookie: Cookie,
	}, nil
}

// Encode serializes the CodecInitCookieBody into raw bytes.
func (s *CodecInitCookieBody) Encode(A byte) []byte {
	// RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
	// Gated fields skip the append on nil pointer / nil slice. Per-
	// field `is_repeat` routes Repeat fields to the dedicated helper.
	// Branch fires before has_vle_fields so a codec mixing VLE +
	// present-if uses the unified encode path.
	r := make([]byte, 0, 68)
	r = append(r, s.Version)
	if s.CookieSize != nil {
		_v := *s.CookieSize
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
