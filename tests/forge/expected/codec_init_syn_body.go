// SCE-MAP: codec_init_syn_body:30

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_init_syn_body

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecInitSynBody represents the codec frame layout.
type CodecInitSynBody struct {
	Version uint8
	SnRes *uint8
	BatchSize *uint16
}

// DecodeCodecInitSynBody decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecInitSynBody(cursor *codec.SceCursor, S byte) (*CodecInitSynBody, error) {
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
		_v := uint16(raw[0])<<8 | uint16(raw[1])
		if err := cursor.Advance(2); err != nil {
			return nil, err
		}
		BatchSize = &_v
	}
	return &CodecInitSynBody{
		Version: Version,
		SnRes: SnRes,
		BatchSize: BatchSize,
	}, nil
}

// Encode serializes the CodecInitSynBody into raw bytes.
func (s *CodecInitSynBody) Encode(S byte) []byte {
	// RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
	// Gated fields skip the append on nil pointer / nil slice. Per-
	// field `is_repeat` routes Repeat fields to the dedicated helper.
	// Branch fires before has_vle_fields so a codec mixing VLE +
	// present-if uses the unified encode path.
	r := make([]byte, 0, 4)
	r = append(r, s.Version)
	if s.SnRes != nil {
		_v := *s.SnRes
		r = append(r, _v)
	}
	if s.BatchSize != nil {
		_v := *s.BatchSize
		r = append(r, byte(_v>>8))
		r = append(r, byte(_v))
	}
	return r
}
