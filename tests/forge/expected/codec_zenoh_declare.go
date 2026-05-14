// SCE-MAP: codec_zenoh_declare:49

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_declare

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_ext_entry"
	"example.com/sce-forge/codec_zenoh_declaration"
)

// CodecZenohDeclare represents the codec frame layout.
type CodecZenohDeclare struct {
	Header uint8
	InterestId *uint32
	Extensions []codec_zenoh_ext_entry.CodecZenohExtEntry
	Declaration codec_zenoh_declaration.CodecZenohDeclaration
}

// DecodeCodecZenohDeclare decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohDeclare(cursor *codec.SceCursor) (*CodecZenohDeclare, error) {
	// RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
	// advances the cursor per field. Gated fields use `*T` for fixed
	// (nil = absent) or `[]byte` (nil = absent) for tail/length-ref;
	// VLE gating uses `*T` like fixed. Per-field `is_repeat` routes
	// Repeat fields to the dedicated helper. Branch fires before
	// has_vle_fields so a codec mixing VLE + present-if uses the
	// unified streaming path.
	var Header uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Header = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var InterestId *uint32
	if (Header & 0x20) != 0 {
		_v, err := cursor.ReadVLEU32()
	if err != nil { return nil, err }
		InterestId = &_v
	}
	var Extensions []codec_zenoh_ext_entry.CodecZenohExtEntry
	if (Header & 0x80) != 0 {
		Extensions = make([]codec_zenoh_ext_entry.CodecZenohExtEntry, 0, 4)
		for _i := 0; _i < int(4); _i++ {
			if cursor.Remaining() == 0 {
				break
			}
			_elem, err := codec_zenoh_ext_entry.DecodeCodecZenohExtEntry(cursor)
			if err != nil {
				return nil, err
			}
			_continue := _elem.Z()
			Extensions = append(Extensions, *_elem)
			if !_continue {
				break
			}
		}
	}
	var Declaration codec_zenoh_declaration.CodecZenohDeclaration
	{
		_emb, err := codec_zenoh_declaration.DecodeCodecZenohDeclaration(cursor)
		if err != nil {
			return nil, err
		}
		Declaration = *_emb
	}
	return &CodecZenohDeclare{
		Header: Header,
		InterestId: InterestId,
		Extensions: Extensions,
		Declaration: Declaration,
	}, nil
}

// RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohDeclare) Mid() uint8 {
	return uint8((s.Header >> 0) & 0x1F)
}

func (s *CodecZenohDeclare) SetMid(v uint8) {
	const _shiftedMask uint8 = 0x1F << 0
	_val := (uint8(v) & 0x1F) << 0
	s.Header = (s.Header &^ _shiftedMask) | _val
}

func (s *CodecZenohDeclare) I() bool {
	return (s.Header & 0x20) != 0
}

func (s *CodecZenohDeclare) SetI(v bool) {
	if v {
		s.Header |= 0x20
	} else {
		s.Header &^= 0x20
	}
}

func (s *CodecZenohDeclare) Z() bool {
	return (s.Header & 0x80) != 0
}

func (s *CodecZenohDeclare) SetZ(v bool) {
	if v {
		s.Header |= 0x80
	} else {
		s.Header &^= 0x80
	}
}

// Encode serializes the CodecZenohDeclare into raw bytes.
func (s *CodecZenohDeclare) Encode() []byte {
	// RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
	// Gated fields skip the append on nil pointer / nil slice. Per-
	// field `is_repeat` routes Repeat fields to the dedicated helper.
	// Branch fires before has_vle_fields so a codec mixing VLE +
	// present-if uses the unified encode path.
	r := make([]byte, 0, 434)
	r = append(r, s.Header)
	if s.InterestId != nil {
		_v := *s.InterestId
	{
		_w := uint64(_v)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	}
	for _, _e := range s.Extensions {
		r = append(r, _e.Encode()...)
	}
	r = append(r, s.Declaration.Encode()...)
	return r
}
