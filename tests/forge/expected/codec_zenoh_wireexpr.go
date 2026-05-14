// SCE-MAP: codec_zenoh_wireexpr:53

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_wireexpr

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"unicode/utf8"
)

// CodecZenohWireexpr represents the codec frame layout.
type CodecZenohWireexpr struct {
	Id uint64
	SuffixLen *uint64
	Suffix *string
}

// DecodeCodecZenohWireexpr decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohWireexpr(cursor *codec.SceCursor, parentFlags byte) (*CodecZenohWireexpr, error) {
	// RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
	// advances the cursor per field. Gated fields use `*T` for fixed
	// (nil = absent) or `[]byte` (nil = absent) for tail/length-ref;
	// VLE gating uses `*T` like fixed. Per-field `is_repeat` routes
	// Repeat fields to the dedicated helper. Branch fires before
	// has_vle_fields so a codec mixing VLE + present-if uses the
	// unified streaming path.
	Id, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var SuffixLen *uint64
	if (parentFlags & 0x20) != 0 {
		_v, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
		SuffixLen = &_v
	}
	var Suffix *string
	if (parentFlags & 0x20) != 0 {
		_n := int(*SuffixLen)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		if !utf8.Valid(raw) {
			return nil, codec.ErrInvalidUTF8
		}
		_v := string(raw)
		Suffix = &_v
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	return &CodecZenohWireexpr{
		Id: Id,
		SuffixLen: SuffixLen,
		Suffix: Suffix,
	}, nil
}

// Encode serializes the CodecZenohWireexpr into raw bytes.
func (s *CodecZenohWireexpr) Encode(parentFlags byte) []byte {
	// RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
	// Gated fields skip the append on nil pointer / nil slice. Per-
	// field `is_repeat` routes Repeat fields to the dedicated helper.
	// Branch fires before has_vle_fields so a codec mixing VLE +
	// present-if uses the unified encode path.
	r := make([]byte, 0, 148)
	{
		_w := uint64(s.Id)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	if s.SuffixLen != nil {
		_v := *s.SuffixLen
	{
		_w := uint64(_v)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	}
	if s.Suffix != nil {
		r = append(r, []byte(*s.Suffix)...)
	}
	return r
}
