// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_ext_entry

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_ext_unit"
	"example.com/sce-forge/codec_zenoh_ext_zint"
	"example.com/sce-forge/codec_zenoh_ext_zbuf"
)

// CodecZenohExtEntryDefault bundles the runtime
// tag value with the catch-all body so encode can round-trip the
// observed tag back onto the wire (RFC §5.B variant primitive B1-β).
type CodecZenohExtEntryDefault struct {
	Tag uint8
	Body codec_zenoh_ext_unit.CodecZenohExtUnit
}

// CodecZenohExtEntryVariant is a discriminated-union body for the codec's
// tag-field suffix (RFC §5.B variant primitive B1-β). Exactly one of
// the pointer fields is non-nil at a time; the active arm is the one
// that matches the current tag value.
type CodecZenohExtEntryVariant struct {
	CodecZenohExtUnit *codec_zenoh_ext_unit.CodecZenohExtUnit
	CodecZenohExtZint *codec_zenoh_ext_zint.CodecZenohExtZint
	CodecZenohExtZbuf *codec_zenoh_ext_zbuf.CodecZenohExtZbuf
	Default *CodecZenohExtEntryDefault
}

// CodecZenohExtEntry represents the codec frame layout.
type CodecZenohExtEntry struct {
	Header uint8
	Body CodecZenohExtEntryVariant
}

// DecodeCodecZenohExtEntry decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohExtEntry(cursor *codec.SceCursor) (*CodecZenohExtEntry, error) {
	// Decode fixed prefix (RFC §5.B variant B1-β: fields before tag suffix).
	raw, err := cursor.PeekSlice(1)
	if err != nil {
		return nil, err
	}
	Header := raw[0]
	if err := cursor.Advance(1); err != nil {
		return nil, err
	}
	// Dispatch on the tag field; each arm decodes its body codec from
	// the cursor. The default arm (when declared) carries the runtime
	// tag value so encode can round-trip it back onto the wire.
	body := CodecZenohExtEntryVariant{}
	switch uint8((Header >> 5) & 0x03) {
	case 0:
		_arm, err := codec_zenoh_ext_unit.DecodeCodecZenohExtUnit(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohExtUnit = _arm
	case 1:
		_arm, err := codec_zenoh_ext_zint.DecodeCodecZenohExtZint(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohExtZint = _arm
	case 2:
		_arm, err := codec_zenoh_ext_zbuf.DecodeCodecZenohExtZbuf(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohExtZbuf = _arm
	default:
		_arm, err := codec_zenoh_ext_unit.DecodeCodecZenohExtUnit(cursor)
		if err != nil {
			return nil, err
		}
		body.Default = &CodecZenohExtEntryDefault{
			Tag: uint8((Header >> 5) & 0x03),
			Body: *_arm,
		}
	}
	return &CodecZenohExtEntry{
		Header: Header,
		Body: body,
	}, nil
}

// RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohExtEntry) Id() uint8 {
	return uint8((s.Header >> 0) & 0x1F)
}

func (s *CodecZenohExtEntry) SetId(v uint8) {
	const _shiftedMask uint8 = 0x1F << 0
	_val := (uint8(v) & 0x1F) << 0
	s.Header = (s.Header &^ _shiftedMask) | _val
}

func (s *CodecZenohExtEntry) Enc() uint8 {
	return uint8((s.Header >> 5) & 0x03)
}

func (s *CodecZenohExtEntry) SetEnc(v uint8) {
	const _shiftedMask uint8 = 0x03 << 5
	_val := (uint8(v) & 0x03) << 5
	s.Header = (s.Header &^ _shiftedMask) | _val
}

// Encode serializes the CodecZenohExtEntry into raw bytes.
func (s *CodecZenohExtEntry) Encode() []byte {
	// Encode fixed prefix (tag field bytes are part of the prefix).
	// The tag value is read from the struct field, NOT derived from
	// the body discriminant — keeping author-set tag / body in sync
	// is the caller's responsibility (v1 keeps the layout simple).
	r := make([]byte, 0, 43)
	r = append(r, byte(s.Header))
	// Append the active arm body's encoded bytes.
	switch {
	case s.Body.CodecZenohExtUnit != nil:
		r = append(r, s.Body.CodecZenohExtUnit.Encode()...)
	case s.Body.CodecZenohExtZint != nil:
		r = append(r, s.Body.CodecZenohExtZint.Encode()...)
	case s.Body.CodecZenohExtZbuf != nil:
		r = append(r, s.Body.CodecZenohExtZbuf.Encode()...)
	case s.Body.Default != nil:
		r = append(r, s.Body.Default.Body.Encode()...)
	}
	return r
}
