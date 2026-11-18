// SCE-MAP: codec_origin_choice:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_origin_choice

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_origin_leaf"
	"example.com/sce-forge/codec_origin_scalar"
)

// CodecOriginChoiceUnknownTag is the tag the catch-all arm of CodecOriginChoice's variant holds: one
// the bits the tag is read from hold and no arm names, held to that where a
// value of it is made (docs/adr/0065 §6 R3, docs/adr/0081 §2), so the catch-all
// a value holds is written and read back as the catch-all. Its zero value
// holds 0x0, the least such tag.
type CodecOriginChoiceUnknownTag struct {
	tag uint8
	set bool
}

// MakeCodecOriginChoiceUnknownTag returns tag as the tag of the catch-all, and false where an
// arm names it or the bits the tag is read from do not hold it.
func MakeCodecOriginChoiceUnknownTag(tag uint8) (CodecOriginChoiceUnknownTag, bool) {
	if tag >= 0x1 && tag <= 0x2 {
		return CodecOriginChoiceUnknownTag{}, false
	}
	return CodecOriginChoiceUnknownTag{tag: tag, set: true}, true
}

// Value returns the tag.
func (t CodecOriginChoiceUnknownTag) Value() uint8 {
	if !t.set {
		return 0x0
	}
	return t.tag
}

// CodecOriginChoiceVariant is the arm of the variant a CodecOriginChoice holds (docs/adr/0081 §2):
// one of CodecOriginChoiceVariantCodecOriginLeaf, CodecOriginChoiceVariantCodecOriginScalar, CodecOriginChoiceVariantDefault, which only this package declares.
type CodecOriginChoiceVariant interface {
	isCodecOriginChoiceVariant()
}

// CodecOriginChoiceVariantCodecOriginLeaf is the arm of CodecOriginChoiceVariant whose body is a CodecOriginLeaf, chosen by the tag 0x1.
type CodecOriginChoiceVariantCodecOriginLeaf struct {
	body codec_origin_leaf.CodecOriginLeafSource
}

// MakeCodecOriginChoiceVariantCodecOriginLeaf returns the arm that holds what it is handed.
func MakeCodecOriginChoiceVariantCodecOriginLeaf(body codec_origin_leaf.CodecOriginLeafSource) CodecOriginChoiceVariantCodecOriginLeaf {
	return CodecOriginChoiceVariantCodecOriginLeaf{body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecOriginLeaf its codec makes by default.
func (a CodecOriginChoiceVariantCodecOriginLeaf) Body() codec_origin_leaf.CodecOriginLeafSource {
	if a.body == nil {
		return &codec_origin_leaf.CodecOriginLeaf{}
	}
	return a.body
}

func (CodecOriginChoiceVariantCodecOriginLeaf) isCodecOriginChoiceVariant() {}

// CodecOriginChoiceVariantCodecOriginScalar is the arm of CodecOriginChoiceVariant whose body is a CodecOriginScalar, chosen by the tag 0x2.
type CodecOriginChoiceVariantCodecOriginScalar struct {
	body codec_origin_scalar.CodecOriginScalarSource
}

// MakeCodecOriginChoiceVariantCodecOriginScalar returns the arm that holds what it is handed.
func MakeCodecOriginChoiceVariantCodecOriginScalar(body codec_origin_scalar.CodecOriginScalarSource) CodecOriginChoiceVariantCodecOriginScalar {
	return CodecOriginChoiceVariantCodecOriginScalar{body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecOriginScalar its codec makes by default.
func (a CodecOriginChoiceVariantCodecOriginScalar) Body() codec_origin_scalar.CodecOriginScalarSource {
	if a.body == nil {
		return &codec_origin_scalar.CodecOriginScalar{}
	}
	return a.body
}

func (CodecOriginChoiceVariantCodecOriginScalar) isCodecOriginChoiceVariant() {}

// CodecOriginChoiceVariantDefault is the arm of CodecOriginChoiceVariant whose body is a CodecOriginScalar, chosen by every tag no arm names.
type CodecOriginChoiceVariantDefault struct {
	tag  CodecOriginChoiceUnknownTag
	body codec_origin_scalar.CodecOriginScalarSource
}

// MakeCodecOriginChoiceVariantDefault returns the arm that holds what it is handed.
func MakeCodecOriginChoiceVariantDefault(tag CodecOriginChoiceUnknownTag, body codec_origin_scalar.CodecOriginScalarSource) CodecOriginChoiceVariantDefault {
	return CodecOriginChoiceVariantDefault{tag: tag, body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecOriginScalar its codec makes by default.
func (a CodecOriginChoiceVariantDefault) Body() codec_origin_scalar.CodecOriginScalarSource {
	if a.body == nil {
		return &codec_origin_scalar.CodecOriginScalar{}
	}
	return a.body
}

// Tag returns the tag the arm holds, which no arm names.
func (a CodecOriginChoiceVariantDefault) Tag() CodecOriginChoiceUnknownTag {
	return a.tag
}

func (CodecOriginChoiceVariantDefault) isCodecOriginChoiceVariant() {}

// CodecOriginChoiceSource is what the encode of CodecOriginChoice reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecOriginChoice is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecOriginChoiceSource interface {
	Body() CodecOriginChoiceVariant
}

// CodecOriginChoice represents the codec frame layout.
type CodecOriginChoice struct {
	body CodecOriginChoiceVariant
}

// MakeCodecOriginChoice returns the CodecOriginChoice that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecOriginChoice, or as its zero value, and is read through its methods.
func MakeCodecOriginChoice(body CodecOriginChoiceVariant) CodecOriginChoice {
	return CodecOriginChoice{body: body}
}

// Body returns the arm of the variant the value holds; a value that
// was handed none, as the zero value, holds CodecOriginChoiceVariantCodecOriginScalar{}.
func (s *CodecOriginChoice) Body() CodecOriginChoiceVariant {
	if s.body == nil {
		return CodecOriginChoiceVariantCodecOriginScalar{}
	}
	return s.body
}

// WireTag returns the tag the frame of the value carries: the tag of the
// arm of its variant it holds (docs/adr/0081 §3).
func (s *CodecOriginChoice) WireTag() uint8 {
	var _tBody uint64
	switch _b := s.Body().(type) {
	case CodecOriginChoiceVariantCodecOriginLeaf:
		_tBody = 0x1
	case CodecOriginChoiceVariantCodecOriginScalar:
		_tBody = 0x2
	case CodecOriginChoiceVariantDefault:
		_tBody = uint64(_b.Tag().Value())
	}
	return uint8(_tBody)
}

// EncodeSource writes any source of CodecOriginChoice into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives tag from
// the arm of the variant the source presents. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecOriginChoiceSource, w codec.SceSink) error {
	_rBody := src.Body()
	var _tBody uint64
	switch _b := _rBody.(type) {
	case CodecOriginChoiceVariantCodecOriginLeaf:
		_tBody = 0x1
	case CodecOriginChoiceVariantCodecOriginScalar:
		_tBody = 0x2
	case CodecOriginChoiceVariantDefault:
		_tBody = uint64(_b.Tag().Value())
	}
	_dTag := uint8(_tBody)
	if err := w.WriteBytes([]byte{byte(_dTag)}); err != nil {
		return err
	}
	switch _b := _rBody.(type) {
	case CodecOriginChoiceVariantCodecOriginLeaf:
		if err := codec_origin_leaf.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	case CodecOriginChoiceVariantCodecOriginScalar:
		if err := codec_origin_scalar.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	case CodecOriginChoiceVariantDefault:
		if err := codec_origin_scalar.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	default:
		panic("CodecOriginChoice: the source presents no arm of its variant")
	}
	return nil
}

// NewCodecOriginChoice returns a CodecOriginChoice initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecOriginChoice().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecOriginChoice{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecOriginChoice() *CodecOriginChoice {
	return &CodecOriginChoice{
	}
}

// DecodeCodecOriginChoice decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecOriginChoice(cursor *codec.SceCursor) (*CodecOriginChoice, error) {
	// Decode fixed prefix (RFC §synth-5-B variant: fields before tag suffix).
	raw, err := cursor.PeekSlice(1)
	if err != nil {
		return nil, err
	}
	Tag := raw[0]
	if err := cursor.Advance(1); err != nil {
		return nil, err
	}
	// Dispatch on the tag field; each arm decodes its body codec from
	// the cursor. The default arm (when declared) carries the runtime
	// tag value so encode can round-trip it back onto the wire.
	var body CodecOriginChoiceVariant
	switch Tag {
	case 1:
		_arm, err := codec_origin_leaf.DecodeCodecOriginLeaf(cursor)
		if err != nil {
			return nil, err
		}
		body = MakeCodecOriginChoiceVariantCodecOriginLeaf(_arm)
	case 2:
		_arm, err := codec_origin_scalar.DecodeCodecOriginScalar(cursor)
		if err != nil {
			return nil, err
		}
		body = MakeCodecOriginChoiceVariantCodecOriginScalar(_arm)
	default:
		_arm, err := codec_origin_scalar.DecodeCodecOriginScalar(cursor)
		if err != nil {
			return nil, err
		}
		_tag := Tag
		body = MakeCodecOriginChoiceVariantDefault(CodecOriginChoiceUnknownTag{tag: _tag, set: true}, _arm)
	}
	return &CodecOriginChoice{
		body: body,
	}, nil
}

// Encode writes the CodecOriginChoice into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecOriginChoice) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecOriginChoice) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 18)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
