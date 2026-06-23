// SCE-MAP: codec_zenoh_declaration:54

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_declaration

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_decl_kexpr"
	"example.com/sce-forge/codec_zenoh_undecl_kexpr"
	"example.com/sce-forge/codec_zenoh_decl_subscriber"
	"example.com/sce-forge/codec_zenoh_undecl_subscriber"
	"example.com/sce-forge/codec_zenoh_decl_queryable"
	"example.com/sce-forge/codec_zenoh_undecl_queryable"
	"example.com/sce-forge/codec_zenoh_decl_token"
	"example.com/sce-forge/codec_zenoh_undecl_token"
	"example.com/sce-forge/codec_zenoh_decl_final"
)

// CodecZenohDeclarationDefault bundles the runtime
// tag value with the catch-all body so encode can round-trip the
// observed tag back onto the wire (RFC §synth-5-B variant primitive).
type CodecZenohDeclarationDefault struct {
	Tag uint8
	Body codec_zenoh_decl_final.CodecZenohDeclFinal
}

// CodecZenohDeclarationVariant is a discriminated-union body for the codec's
// tag-field suffix (RFC §synth-5-B variant primitive). Exactly one of
// the pointer fields is non-nil at a time; the active arm is the one
// that matches the current tag value.
type CodecZenohDeclarationVariant struct {
	CodecZenohDeclKexpr *codec_zenoh_decl_kexpr.CodecZenohDeclKexpr
	CodecZenohUndeclKexpr *codec_zenoh_undecl_kexpr.CodecZenohUndeclKexpr
	CodecZenohDeclSubscriber *codec_zenoh_decl_subscriber.CodecZenohDeclSubscriber
	CodecZenohUndeclSubscriber *codec_zenoh_undecl_subscriber.CodecZenohUndeclSubscriber
	CodecZenohDeclQueryable *codec_zenoh_decl_queryable.CodecZenohDeclQueryable
	CodecZenohUndeclQueryable *codec_zenoh_undecl_queryable.CodecZenohUndeclQueryable
	CodecZenohDeclToken *codec_zenoh_decl_token.CodecZenohDeclToken
	CodecZenohUndeclToken *codec_zenoh_undecl_token.CodecZenohUndeclToken
	CodecZenohDeclFinal *codec_zenoh_decl_final.CodecZenohDeclFinal
	Default *CodecZenohDeclarationDefault
}

// CodecZenohDeclaration represents the codec frame layout.
type CodecZenohDeclaration struct {
	Header uint8
	Body CodecZenohDeclarationVariant
}

// NewCodecZenohDeclaration returns a CodecZenohDeclaration initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecZenohDeclaration().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecZenohDeclaration{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecZenohDeclaration() *CodecZenohDeclaration {
	return &CodecZenohDeclaration{
		Body: CodecZenohDeclarationVariant{
			CodecZenohDeclFinal: &codec_zenoh_decl_final.CodecZenohDeclFinal{},
		},
	}
}

// DecodeCodecZenohDeclaration decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohDeclaration(cursor *codec.SceCursor) (*CodecZenohDeclaration, error) {
	// Decode fixed prefix (RFC §synth-5-B variant: fields before tag suffix).
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
		_arm, err := codec_zenoh_decl_kexpr.DecodeCodecZenohDeclKexpr(cursor, byte((Header >> 5) & 0x1))
		if err != nil {
			return nil, err
		}
		body.CodecZenohDeclKexpr = _arm
	case 1:
		_arm, err := codec_zenoh_undecl_kexpr.DecodeCodecZenohUndeclKexpr(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohUndeclKexpr = _arm
	case 2:
		_arm, err := codec_zenoh_decl_subscriber.DecodeCodecZenohDeclSubscriber(cursor, byte((Header >> 5) & 0x1))
		if err != nil {
			return nil, err
		}
		body.CodecZenohDeclSubscriber = _arm
	case 3:
		_arm, err := codec_zenoh_undecl_subscriber.DecodeCodecZenohUndeclSubscriber(cursor, byte((Header >> 7) & 0x1))
		if err != nil {
			return nil, err
		}
		body.CodecZenohUndeclSubscriber = _arm
	case 4:
		_arm, err := codec_zenoh_decl_queryable.DecodeCodecZenohDeclQueryable(cursor, byte((Header >> 5) & 0x1), byte((Header >> 7) & 0x1))
		if err != nil {
			return nil, err
		}
		body.CodecZenohDeclQueryable = _arm
	case 5:
		_arm, err := codec_zenoh_undecl_queryable.DecodeCodecZenohUndeclQueryable(cursor, byte((Header >> 7) & 0x1))
		if err != nil {
			return nil, err
		}
		body.CodecZenohUndeclQueryable = _arm
	case 6:
		_arm, err := codec_zenoh_decl_token.DecodeCodecZenohDeclToken(cursor, byte((Header >> 5) & 0x1))
		if err != nil {
			return nil, err
		}
		body.CodecZenohDeclToken = _arm
	case 7:
		_arm, err := codec_zenoh_undecl_token.DecodeCodecZenohUndeclToken(cursor, byte((Header >> 7) & 0x1))
		if err != nil {
			return nil, err
		}
		body.CodecZenohUndeclToken = _arm
	case 26:
		_arm, err := codec_zenoh_decl_final.DecodeCodecZenohDeclFinal(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohDeclFinal = _arm
	default:
		_arm, err := codec_zenoh_decl_final.DecodeCodecZenohDeclFinal(cursor)
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

// RFC §synth-5-B flags primitive: per-bit-range accessors over
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

// Encode writes the CodecZenohDeclaration into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohDeclaration) Encode(w codec.SceSink) error {
	// Encode fixed prefix (tag field bytes are part of the prefix).
	if err := w.WriteBytes([]byte{ byte(s.Header) }); err != nil {
		return err
	}
	// Append the active arm body's encoded bytes via the same sink.
	switch {
	case s.Body.CodecZenohDeclKexpr != nil:
		if err := s.Body.CodecZenohDeclKexpr.Encode(w, byte((s.Header >> 5) & 0x1)); err != nil {
			return err
		}
	case s.Body.CodecZenohUndeclKexpr != nil:
		if err := s.Body.CodecZenohUndeclKexpr.Encode(w); err != nil {
			return err
		}
	case s.Body.CodecZenohDeclSubscriber != nil:
		if err := s.Body.CodecZenohDeclSubscriber.Encode(w, byte((s.Header >> 5) & 0x1)); err != nil {
			return err
		}
	case s.Body.CodecZenohUndeclSubscriber != nil:
		if err := s.Body.CodecZenohUndeclSubscriber.Encode(w, byte((s.Header >> 7) & 0x1)); err != nil {
			return err
		}
	case s.Body.CodecZenohDeclQueryable != nil:
		if err := s.Body.CodecZenohDeclQueryable.Encode(w, byte((s.Header >> 5) & 0x1), byte((s.Header >> 7) & 0x1)); err != nil {
			return err
		}
	case s.Body.CodecZenohUndeclQueryable != nil:
		if err := s.Body.CodecZenohUndeclQueryable.Encode(w, byte((s.Header >> 7) & 0x1)); err != nil {
			return err
		}
	case s.Body.CodecZenohDeclToken != nil:
		if err := s.Body.CodecZenohDeclToken.Encode(w, byte((s.Header >> 5) & 0x1)); err != nil {
			return err
		}
	case s.Body.CodecZenohUndeclToken != nil:
		if err := s.Body.CodecZenohUndeclToken.Encode(w, byte((s.Header >> 7) & 0x1)); err != nil {
			return err
		}
	case s.Body.CodecZenohDeclFinal != nil:
		if err := s.Body.CodecZenohDeclFinal.Encode(w); err != nil {
			return err
		}
	case s.Body.Default != nil:
		if err := s.Body.Default.Body.Encode(w); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohDeclaration) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 274)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
