// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_declaration

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_decl_keyexpr"
	"example.com/sce-forge/codec_zenoh_undecl_keyexpr"
	"example.com/sce-forge/codec_zenoh_decl_subscriber"
	"example.com/sce-forge/codec_zenoh_undecl_subscriber"
	"example.com/sce-forge/codec_zenoh_decl_queryable"
	"example.com/sce-forge/codec_zenoh_undecl_queryable"
	"example.com/sce-forge/codec_zenoh_decl_token"
	"example.com/sce-forge/codec_zenoh_undecl_token"
	"example.com/sce-forge/codec_decl_final"
)

// CodecZenohDeclarationDefault bundles the runtime
// tag value with the catch-all body so encode can round-trip the
// observed tag back onto the wire (RFC §5.B variant primitive B1-β).
type CodecZenohDeclarationDefault struct {
	Tag uint8
	Body codec_decl_final.CodecDeclFinal
}

// CodecZenohDeclarationVariant is a discriminated-union body for the codec's
// tag-field suffix (RFC §5.B variant primitive B1-β). Exactly one of
// the pointer fields is non-nil at a time; the active arm is the one
// that matches the current tag value.
type CodecZenohDeclarationVariant struct {
	CodecZenohDeclKeyexpr *codec_zenoh_decl_keyexpr.CodecZenohDeclKeyexpr
	CodecZenohUndeclKeyexpr *codec_zenoh_undecl_keyexpr.CodecZenohUndeclKeyexpr
	CodecZenohDeclSubscriber *codec_zenoh_decl_subscriber.CodecZenohDeclSubscriber
	CodecZenohUndeclSubscriber *codec_zenoh_undecl_subscriber.CodecZenohUndeclSubscriber
	CodecZenohDeclQueryable *codec_zenoh_decl_queryable.CodecZenohDeclQueryable
	CodecZenohUndeclQueryable *codec_zenoh_undecl_queryable.CodecZenohUndeclQueryable
	CodecZenohDeclToken *codec_zenoh_decl_token.CodecZenohDeclToken
	CodecZenohUndeclToken *codec_zenoh_undecl_token.CodecZenohUndeclToken
	CodecDeclFinal *codec_decl_final.CodecDeclFinal
	Default *CodecZenohDeclarationDefault
}

// CodecZenohDeclaration represents the codec frame layout.
type CodecZenohDeclaration struct {
	Header uint8
	Body CodecZenohDeclarationVariant
}

// DecodeCodecZenohDeclaration decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohDeclaration(cursor *codec.SceCursor) (*CodecZenohDeclaration, error) {
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
	body := CodecZenohDeclarationVariant{}
	switch uint8((Header >> 0) & 0x1F) {
	case 0:
		_arm, err := codec_zenoh_decl_keyexpr.DecodeCodecZenohDeclKeyexpr(cursor, Header)
		if err != nil {
			return nil, err
		}
		body.CodecZenohDeclKeyexpr = _arm
	case 1:
		_arm, err := codec_zenoh_undecl_keyexpr.DecodeCodecZenohUndeclKeyexpr(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohUndeclKeyexpr = _arm
	case 2:
		_arm, err := codec_zenoh_decl_subscriber.DecodeCodecZenohDeclSubscriber(cursor, Header)
		if err != nil {
			return nil, err
		}
		body.CodecZenohDeclSubscriber = _arm
	case 3:
		_arm, err := codec_zenoh_undecl_subscriber.DecodeCodecZenohUndeclSubscriber(cursor, Header)
		if err != nil {
			return nil, err
		}
		body.CodecZenohUndeclSubscriber = _arm
	case 4:
		_arm, err := codec_zenoh_decl_queryable.DecodeCodecZenohDeclQueryable(cursor, Header)
		if err != nil {
			return nil, err
		}
		body.CodecZenohDeclQueryable = _arm
	case 5:
		_arm, err := codec_zenoh_undecl_queryable.DecodeCodecZenohUndeclQueryable(cursor, Header)
		if err != nil {
			return nil, err
		}
		body.CodecZenohUndeclQueryable = _arm
	case 6:
		_arm, err := codec_zenoh_decl_token.DecodeCodecZenohDeclToken(cursor, Header)
		if err != nil {
			return nil, err
		}
		body.CodecZenohDeclToken = _arm
	case 7:
		_arm, err := codec_zenoh_undecl_token.DecodeCodecZenohUndeclToken(cursor, Header)
		if err != nil {
			return nil, err
		}
		body.CodecZenohUndeclToken = _arm
	case 26:
		_arm, err := codec_decl_final.DecodeCodecDeclFinal(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecDeclFinal = _arm
	default:
		_arm, err := codec_decl_final.DecodeCodecDeclFinal(cursor)
		if err != nil {
			return nil, err
		}
		body.Default = &CodecZenohDeclarationDefault{
			Tag: uint8((Header >> 0) & 0x1F),
			Body: *_arm,
		}
	}
	return &CodecZenohDeclaration{
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
func (s *CodecZenohDeclaration) Mid() uint8 {
	return uint8((s.Header >> 0) & 0x1F)
}

func (s *CodecZenohDeclaration) SetMid(v uint8) {
	const _shiftedMask uint8 = 0x1F << 0
	_val := (uint8(v) & 0x1F) << 0
	s.Header = (s.Header &^ _shiftedMask) | _val
}

func (s *CodecZenohDeclaration) N() bool {
	return (s.Header & 0x20) != 0
}

func (s *CodecZenohDeclaration) SetN(v bool) {
	if v {
		s.Header |= 0x20
	} else {
		s.Header &^= 0x20
	}
}

func (s *CodecZenohDeclaration) M() bool {
	return (s.Header & 0x40) != 0
}

func (s *CodecZenohDeclaration) SetM(v bool) {
	if v {
		s.Header |= 0x40
	} else {
		s.Header &^= 0x40
	}
}

func (s *CodecZenohDeclaration) Z() bool {
	return (s.Header & 0x80) != 0
}

func (s *CodecZenohDeclaration) SetZ(v bool) {
	if v {
		s.Header |= 0x80
	} else {
		s.Header &^= 0x80
	}
}

// Encode serializes the CodecZenohDeclaration into raw bytes.
func (s *CodecZenohDeclaration) Encode() []byte {
	// Encode fixed prefix (tag field bytes are part of the prefix).
	// The tag value is read from the struct field, NOT derived from
	// the body discriminant — keeping author-set tag / body in sync
	// is the caller's responsibility (v1 keeps the layout simple).
	r := make([]byte, 0, 275)
	r = append(r, byte(s.Header))
	// Append the active arm body's encoded bytes.
	switch {
	case s.Body.CodecZenohDeclKeyexpr != nil:
		r = append(r, s.Body.CodecZenohDeclKeyexpr.Encode(s.Header)...)
	case s.Body.CodecZenohUndeclKeyexpr != nil:
		r = append(r, s.Body.CodecZenohUndeclKeyexpr.Encode()...)
	case s.Body.CodecZenohDeclSubscriber != nil:
		r = append(r, s.Body.CodecZenohDeclSubscriber.Encode(s.Header)...)
	case s.Body.CodecZenohUndeclSubscriber != nil:
		r = append(r, s.Body.CodecZenohUndeclSubscriber.Encode(s.Header)...)
	case s.Body.CodecZenohDeclQueryable != nil:
		r = append(r, s.Body.CodecZenohDeclQueryable.Encode(s.Header)...)
	case s.Body.CodecZenohUndeclQueryable != nil:
		r = append(r, s.Body.CodecZenohUndeclQueryable.Encode(s.Header)...)
	case s.Body.CodecZenohDeclToken != nil:
		r = append(r, s.Body.CodecZenohDeclToken.Encode(s.Header)...)
	case s.Body.CodecZenohUndeclToken != nil:
		r = append(r, s.Body.CodecZenohUndeclToken.Encode(s.Header)...)
	case s.Body.CodecDeclFinal != nil:
		r = append(r, s.Body.CodecDeclFinal.Encode()...)
	case s.Body.Default != nil:
		r = append(r, s.Body.Default.Body.Encode()...)
	}
	return r
}
