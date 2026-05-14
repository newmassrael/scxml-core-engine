// SCE-MAP: codec_ext_encoding_info:44

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_ext_encoding_info

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecExtEncodingInfo represents the codec frame layout.
type CodecExtEncodingInfo struct {
	CombinedId uint32
	SchemaSize uint8
	Schema []byte
}

// DecodeCodecExtEncodingInfo decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecExtEncodingInfo(cursor *codec.SceCursor) (*CodecExtEncodingInfo, error) {
	// RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
	// advances the cursor per field. Gated fields use `*T` for fixed
	// (nil = absent) or `[]byte` (nil = absent) for tail/length-ref;
	// VLE gating uses `*T` like fixed. Per-field `is_repeat` routes
	// Repeat fields to the dedicated helper. Branch fires before
	// has_vle_fields so a codec mixing VLE + present-if uses the
	// unified streaming path.
	CombinedId, err := cursor.ReadVLEU32()
	if err != nil { return nil, err }
	var SchemaSize uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		SchemaSize = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var Schema []byte
	if (CombinedId & 0x00000001) != 0 {
		_n := int(SchemaSize)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Schema = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	return &CodecExtEncodingInfo{
		CombinedId: CombinedId,
		SchemaSize: SchemaSize,
		Schema: Schema,
	}, nil
}

// RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecExtEncodingInfo) HasSchema() bool {
	return (s.CombinedId & 0x00000001) != 0
}

func (s *CodecExtEncodingInfo) SetHasSchema(v bool) {
	if v {
		s.CombinedId |= 0x00000001
	} else {
		s.CombinedId &^= 0x00000001
	}
}

// Encode serializes the CodecExtEncodingInfo into raw bytes.
func (s *CodecExtEncodingInfo) Encode() []byte {
	// RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
	// Gated fields skip the append on nil pointer / nil slice. Per-
	// field `is_repeat` routes Repeat fields to the dedicated helper.
	// Branch fires before has_vle_fields so a codec mixing VLE +
	// present-if uses the unified encode path.
	r := make([]byte, 0, 71)
	{
		_w := uint64(s.CombinedId)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	r = append(r, s.SchemaSize)
	if s.Schema != nil {
		r = append(r, s.Schema...)
	}
	return r
}
