// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_undecl_subscriber

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_decl_ext_keyexpr"
)

// CodecZenohUndeclSubscriber represents the codec frame layout.
type CodecZenohUndeclSubscriber struct {
	Id uint32
	ExtKeyexpr *codec_zenoh_decl_ext_keyexpr.CodecZenohDeclExtKeyexpr
}

// DecodeCodecZenohUndeclSubscriber decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohUndeclSubscriber(cursor *codec.SceCursor, parentFlags byte) (*CodecZenohUndeclSubscriber, error) {
	// RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
	// advances the cursor per field. Gated fields use `*T` for fixed
	// (nil = absent) or `[]byte` (nil = absent) for tail/length-ref;
	// VLE gating uses `*T` like fixed. Per-field `is_repeat` routes
	// Repeat fields to the dedicated helper. Branch fires before
	// has_vle_fields so a codec mixing VLE + present-if uses the
	// unified streaming path.
	Id, err := cursor.ReadVLEU32()
	if err != nil { return nil, err }
	var ExtKeyexpr *codec_zenoh_decl_ext_keyexpr.CodecZenohDeclExtKeyexpr
	if (parentFlags & 0x80) != 0 {
		_emb, err := codec_zenoh_decl_ext_keyexpr.DecodeCodecZenohDeclExtKeyexpr(cursor)
		if err != nil {
			return nil, err
		}
		ExtKeyexpr = _emb
	}
	return &CodecZenohUndeclSubscriber{
		Id: Id,
		ExtKeyexpr: ExtKeyexpr,
	}, nil
}

// Encode serializes the CodecZenohUndeclSubscriber into raw bytes.
func (s *CodecZenohUndeclSubscriber) Encode(parentFlags byte) []byte {
	// RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
	// Gated fields skip the append on nil pointer / nil slice. Per-
	// field `is_repeat` routes Repeat fields to the dedicated helper.
	// Branch fires before has_vle_fields so a codec mixing VLE +
	// present-if uses the unified encode path.
	r := make([]byte, 0, 261)
	{
		_w := uint64(s.Id)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	if (parentFlags & 0x80) != 0 {
		if s.ExtKeyexpr != nil {
			r = append(r, s.ExtKeyexpr.Encode()...)
		}
	}
	return r
}
