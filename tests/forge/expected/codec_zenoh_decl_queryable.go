// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_decl_queryable

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_wireexpr"
)

// CodecZenohDeclQueryable represents the codec frame layout.
type CodecZenohDeclQueryable struct {
	Id uint32
	Wireexpr codec_zenoh_wireexpr.CodecZenohWireexpr
	ExtType *uint8
	ExtValue *uint64
}

// DecodeCodecZenohDeclQueryable decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohDeclQueryable(cursor *codec.SceCursor, parentFlags byte) (*CodecZenohDeclQueryable, error) {
	// RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
	// advances the cursor per field. Gated fields use `*T` for fixed
	// (nil = absent) or `[]byte` (nil = absent) for tail/length-ref;
	// VLE gating uses `*T` like fixed. Per-field `is_repeat` routes
	// Repeat fields to the dedicated helper. Branch fires before
	// has_vle_fields so a codec mixing VLE + present-if uses the
	// unified streaming path.
	Id, err := cursor.ReadVLEU32()
	if err != nil { return nil, err }
	var Wireexpr codec_zenoh_wireexpr.CodecZenohWireexpr
	{
		_emb, err := codec_zenoh_wireexpr.DecodeCodecZenohWireexpr(cursor, parentFlags)
		if err != nil {
			return nil, err
		}
		Wireexpr = *_emb
	}
	var ExtType *uint8
	if (parentFlags & 0x80) != 0 {
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		_v := raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
		ExtType = &_v
	}
	var ExtValue *uint64
	if (parentFlags & 0x80) != 0 {
		_v, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
		ExtValue = &_v
	}
	return &CodecZenohDeclQueryable{
		Id: Id,
		Wireexpr: Wireexpr,
		ExtType: ExtType,
		ExtValue: ExtValue,
	}, nil
}

// Encode serializes the CodecZenohDeclQueryable into raw bytes.
func (s *CodecZenohDeclQueryable) Encode(parentFlags byte) []byte {
	// RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
	// Gated fields skip the append on nil pointer / nil slice. Per-
	// field `is_repeat` routes Repeat fields to the dedicated helper.
	// Branch fires before has_vle_fields so a codec mixing VLE +
	// present-if uses the unified encode path.
	r := make([]byte, 0, 274)
	{
		_w := uint64(s.Id)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	r = append(r, s.Wireexpr.Encode(parentFlags)...)
	if s.ExtType != nil {
		_v := *s.ExtType
		r = append(r, _v)
	}
	if s.ExtValue != nil {
		_v := *s.ExtValue
	{
		_w := uint64(_v)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	}
	return r
}
