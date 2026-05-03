// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_decl_ext_keyexpr_inner

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohDeclExtKeyexprInner represents the codec frame layout.
type CodecZenohDeclExtKeyexprInner struct {
	InnerHeader uint8
	Id uint64
	Suffix []byte
}

// DecodeCodecZenohDeclExtKeyexprInner decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohDeclExtKeyexprInner(cursor *codec.SceCursor) (*CodecZenohDeclExtKeyexprInner, error) {
	// RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
	// advances the cursor per field. Gated fields use `*T` for fixed
	// (nil = absent) or `[]byte` (nil = absent) for tail/length-ref;
	// VLE gating uses `*T` like fixed. Per-field `is_repeat` routes
	// Repeat fields to the dedicated helper. Branch fires before
	// has_vle_fields so a codec mixing VLE + present-if uses the
	// unified streaming path.
	var InnerHeader uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		InnerHeader = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	Id, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var Suffix []byte
	if (InnerHeader & 0x01) != 0 {
		_n := cursor.Remaining()
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Suffix = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	return &CodecZenohDeclExtKeyexprInner{
		InnerHeader: InnerHeader,
		Id: Id,
		Suffix: Suffix,
	}, nil
}

// RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohDeclExtKeyexprInner) N() bool {
	return (s.InnerHeader & 0x01) != 0
}

func (s *CodecZenohDeclExtKeyexprInner) SetN(v bool) {
	if v {
		s.InnerHeader |= 0x01
	} else {
		s.InnerHeader &^= 0x01
	}
}

func (s *CodecZenohDeclExtKeyexprInner) M() bool {
	return (s.InnerHeader & 0x02) != 0
}

func (s *CodecZenohDeclExtKeyexprInner) SetM(v bool) {
	if v {
		s.InnerHeader |= 0x02
	} else {
		s.InnerHeader &^= 0x02
	}
}

// Encode serializes the CodecZenohDeclExtKeyexprInner into raw bytes.
func (s *CodecZenohDeclExtKeyexprInner) Encode() []byte {
	// RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
	// Gated fields skip the append on nil pointer / nil slice. Per-
	// field `is_repeat` routes Repeat fields to the dedicated helper.
	// Branch fires before has_vle_fields so a codec mixing VLE +
	// present-if uses the unified encode path.
	r := make([]byte, 0, 139)
	r = append(r, s.InnerHeader)
	{
		_w := uint64(s.Id)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	if s.Suffix != nil {
		r = append(r, s.Suffix...)
	}
	return r
}
