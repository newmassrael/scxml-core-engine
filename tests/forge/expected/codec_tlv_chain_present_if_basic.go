// SCE-MAP: codec_tlv_chain_present_if_basic:37

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_tlv_chain_present_if_basic

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_tlv_entry"
)

// CodecTlvChainPresentIfBasic represents the codec frame layout.
type CodecTlvChainPresentIfBasic struct {
	Carrier uint8
	Entries []codec_tlv_entry.CodecTlvEntry
}

// DecodeCodecTlvChainPresentIfBasic decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecTlvChainPresentIfBasic(cursor *codec.SceCursor) (*CodecTlvChainPresentIfBasic, error) {
	// RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
	// advances the cursor per field. Gated fields use `*T` for fixed
	// (nil = absent) or `[]byte` (nil = absent) for tail/length-ref;
	// VLE gating uses `*T` like fixed. Per-field `is_repeat` routes
	// Repeat fields to the dedicated helper. Branch fires before
	// has_vle_fields so a codec mixing VLE + present-if uses the
	// unified streaming path.
	var Carrier uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Carrier = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var Entries []codec_tlv_entry.CodecTlvEntry
	if (Carrier & 0x01) != 0 {
		Entries = make([]codec_tlv_entry.CodecTlvEntry, 0, 4)
		for _i := 0; _i < int(4); _i++ {
			if cursor.Remaining() == 0 {
				break
			}
			_elem, err := codec_tlv_entry.DecodeCodecTlvEntry(cursor)
			if err != nil {
				return nil, err
			}
			Entries = append(Entries, *_elem)
		}
		if cursor.Remaining() > 0 {
			return nil, codec.ErrTlvChainOverflow
		}
	}
	return &CodecTlvChainPresentIfBasic{
		Carrier: Carrier,
		Entries: Entries,
	}, nil
}

// RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecTlvChainPresentIfBasic) HasChain() bool {
	return (s.Carrier & 0x01) != 0
}

func (s *CodecTlvChainPresentIfBasic) SetHasChain(v bool) {
	if v {
		s.Carrier |= 0x01
	} else {
		s.Carrier &^= 0x01
	}
}

// Encode serializes the CodecTlvChainPresentIfBasic into raw bytes.
func (s *CodecTlvChainPresentIfBasic) Encode() []byte {
	// RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
	// Gated fields skip the append on nil pointer / nil slice. Per-
	// field `is_repeat` routes Repeat fields to the dedicated helper.
	// Branch fires before has_vle_fields so a codec mixing VLE +
	// present-if uses the unified encode path.
	r := make([]byte, 0, 137)
	r = append(r, s.Carrier)
	for _, _e := range s.Entries {
		r = append(r, _e.Encode()...)
	}
	return r
}
