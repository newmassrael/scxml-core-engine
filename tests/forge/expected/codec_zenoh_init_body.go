// SCE-MAP: codec_zenoh_init_body:42

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_init_body

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohInitBody represents the codec frame layout.
type CodecZenohInitBody struct {
	Version uint8
	Cbyte uint8
	Zid []byte
	SnRes *uint8
	BatchSize *uint16
	CookieLen *uint64
	Cookie []byte
}

// DecodeCodecZenohInitBody decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohInitBody(cursor *codec.SceCursor, S byte, A byte) (*CodecZenohInitBody, error) {
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
	var Cbyte uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Cbyte = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var Zid []byte
	{
		_n := (int((Cbyte >> 4) & 0xF) + 1)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Zid = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	var SnRes *uint8
	if (S & 0x01) != 0 {
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		_v := raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
		SnRes = &_v
	}
	var BatchSize *uint16
	if (S & 0x01) != 0 {
		raw, err := cursor.PeekSlice(2)
		if err != nil {
			return nil, err
		}
		_v := uint16(raw[0]) | uint16(raw[1])<<8
		if err := cursor.Advance(2); err != nil {
			return nil, err
		}
		BatchSize = &_v
	}
	var CookieLen *uint64
	if (A & 0x01) != 0 {
		_v, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
		CookieLen = &_v
	}
	var Cookie []byte
	if (A & 0x01) != 0 {
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
	return &CodecZenohInitBody{
		Version: Version,
		Cbyte: Cbyte,
		Zid: Zid,
		SnRes: SnRes,
		BatchSize: BatchSize,
		CookieLen: CookieLen,
		Cookie: Cookie,
	}, nil
}

// RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohInitBody) Whatami() uint8 {
	return uint8((s.Cbyte >> 0) & 0x03)
}

func (s *CodecZenohInitBody) SetWhatami(v uint8) {
	const _shiftedMask uint8 = 0x03 << 0
	_val := (uint8(v) & 0x03) << 0
	s.Cbyte = (s.Cbyte &^ _shiftedMask) | _val
}

func (s *CodecZenohInitBody) ZidLenM1() uint8 {
	return uint8((s.Cbyte >> 4) & 0x0F)
}

func (s *CodecZenohInitBody) SetZidLenM1(v uint8) {
	const _shiftedMask uint8 = 0x0F << 4
	_val := (uint8(v) & 0x0F) << 4
	s.Cbyte = (s.Cbyte &^ _shiftedMask) | _val
}

// Encode serializes the CodecZenohInitBody into raw bytes.
func (s *CodecZenohInitBody) Encode(S byte, A byte) []byte {
	// RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
	// Gated fields skip the append on nil pointer / nil slice. Per-
	// field `is_repeat` routes Repeat fields to the dedicated helper.
	// Branch fires before has_vle_fields so a codec mixing VLE +
	// present-if uses the unified encode path.
	r := make([]byte, 0, 160)
	r = append(r, s.Version)
	r = append(r, s.Cbyte)
	r = append(r, s.Zid...)
	if s.SnRes != nil {
		_v := *s.SnRes
		r = append(r, _v)
	}
	if s.BatchSize != nil {
		_v := *s.BatchSize
		r = append(r, byte(_v))
		r = append(r, byte(_v>>8))
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
