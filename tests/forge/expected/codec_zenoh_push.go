// SCE-MAP: codec_zenoh_push:45 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_push

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_push_body"
)

// CodecZenohPushUnknownTag is the tag the catch-all arm of CodecZenohPush's variant holds: one
// the bits the tag is read from hold and no arm names, held to that where a
// value of it is made (docs/adr/0065 §6 R3, docs/adr/0081 §2), so the catch-all
// a value holds is written and read back as the catch-all. Its zero value
// holds 0x0, the least such tag.
type CodecZenohPushUnknownTag struct {
	tag uint8
	set bool
}

// MakeCodecZenohPushUnknownTag returns tag as the tag of the catch-all, and false where an
// arm names it or the bits the tag is read from do not hold it.
func MakeCodecZenohPushUnknownTag(tag uint8) (CodecZenohPushUnknownTag, bool) {
	if tag == 0x1D || tag > 0x1F {
		return CodecZenohPushUnknownTag{}, false
	}
	return CodecZenohPushUnknownTag{tag: tag, set: true}, true
}

// Value returns the tag.
func (t CodecZenohPushUnknownTag) Value() uint8 {
	if !t.set {
		return 0x0
	}
	return t.tag
}

// CodecZenohPushVariant is the arm of the variant a CodecZenohPush holds (docs/adr/0081 §2):
// one of CodecZenohPushVariantCodecZenohPushBody, CodecZenohPushVariantDefault, which only this package declares.
type CodecZenohPushVariant interface {
	isCodecZenohPushVariant()
}

// CodecZenohPushVariantCodecZenohPushBody is the arm of CodecZenohPushVariant whose body is a CodecZenohPushBody, chosen by the tag 0x1D.
type CodecZenohPushVariantCodecZenohPushBody struct {
	body codec_zenoh_push_body.CodecZenohPushBodySource
}

// MakeCodecZenohPushVariantCodecZenohPushBody returns the arm that holds what it is handed.
func MakeCodecZenohPushVariantCodecZenohPushBody(body codec_zenoh_push_body.CodecZenohPushBodySource) CodecZenohPushVariantCodecZenohPushBody {
	return CodecZenohPushVariantCodecZenohPushBody{body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecZenohPushBody its codec makes by default.
func (a CodecZenohPushVariantCodecZenohPushBody) Body() codec_zenoh_push_body.CodecZenohPushBodySource {
	if a.body == nil {
		return codec_zenoh_push_body.NewCodecZenohPushBody()
	}
	return a.body
}

func (CodecZenohPushVariantCodecZenohPushBody) isCodecZenohPushVariant() {}

// CodecZenohPushVariantDefault is the arm of CodecZenohPushVariant whose body is a CodecZenohPushBody, chosen by every tag no arm names.
type CodecZenohPushVariantDefault struct {
	tag  CodecZenohPushUnknownTag
	body codec_zenoh_push_body.CodecZenohPushBodySource
}

// MakeCodecZenohPushVariantDefault returns the arm that holds what it is handed.
func MakeCodecZenohPushVariantDefault(tag CodecZenohPushUnknownTag, body codec_zenoh_push_body.CodecZenohPushBodySource) CodecZenohPushVariantDefault {
	return CodecZenohPushVariantDefault{tag: tag, body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecZenohPushBody its codec makes by default.
func (a CodecZenohPushVariantDefault) Body() codec_zenoh_push_body.CodecZenohPushBodySource {
	if a.body == nil {
		return codec_zenoh_push_body.NewCodecZenohPushBody()
	}
	return a.body
}

// Tag returns the tag the arm holds, which no arm names.
func (a CodecZenohPushVariantDefault) Tag() CodecZenohPushUnknownTag {
	return a.tag
}

func (CodecZenohPushVariantDefault) isCodecZenohPushVariant() {}

// CodecZenohPushSource is what the encode of CodecZenohPush reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohPush is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohPushSource interface {
	Header() uint8
	Body() CodecZenohPushVariant
}

// CodecZenohPush represents the codec frame layout.
type CodecZenohPush struct {
	header uint8
	body CodecZenohPushVariant
}

// MakeCodecZenohPush returns the CodecZenohPush that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohPush, or as its zero value, and is read through its methods.
func MakeCodecZenohPush(header uint8, body CodecZenohPushVariant) CodecZenohPush {
	return CodecZenohPush{header: header, body: body}
}

// Header returns the element the value presents as Header.
func (s *CodecZenohPush) Header() uint8 {
	return s.header
}

// Body returns the arm of the variant the value holds; a value that
// was handed none, as the zero value, holds CodecZenohPushVariantCodecZenohPushBody{}.
func (s *CodecZenohPush) Body() CodecZenohPushVariant {
	if s.body == nil {
		return CodecZenohPushVariantCodecZenohPushBody{}
	}
	return s.body
}

// EncodeSource writes any source of CodecZenohPush into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohPushSource, w codec.SceSink) error {
	_rBody := src.Body()
	var _tBody uint64
	switch _b := _rBody.(type) {
	case CodecZenohPushVariantCodecZenohPushBody:
		_tBody = 0x1D
	case CodecZenohPushVariantDefault:
		_tBody = uint64(_b.Tag().Value())
	}
	_cHeader := src.Header()
	_cHeader = (_cHeader &^ 0x1F) | (uint8(_tBody & 0x1F) << 0)
	if err := w.WriteBytes([]byte{byte(_cHeader)}); err != nil {
		return err
	}
	switch _b := _rBody.(type) {
	case CodecZenohPushVariantCodecZenohPushBody:
		if err := codec_zenoh_push_body.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	case CodecZenohPushVariantDefault:
		if err := codec_zenoh_push_body.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	default:
		panic("CodecZenohPush: the source presents no arm of its variant")
	}
	return nil
}

// NewCodecZenohPush returns a CodecZenohPush initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecZenohPush().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecZenohPush{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecZenohPush() *CodecZenohPush {
	return &CodecZenohPush{
		header: uint8(0x1d),
	}
}

// DecodeCodecZenohPush decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohPush(cursor *codec.SceCursor) (*CodecZenohPush, error) {
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
	var body CodecZenohPushVariant
	switch uint8((Header >> 0) & 0x1F) {
	case 29:
		_arm, err := codec_zenoh_push_body.DecodeCodecZenohPushBody(cursor)
		if err != nil {
			return nil, err
		}
		body = MakeCodecZenohPushVariantCodecZenohPushBody(_arm)
	default:
		_arm, err := codec_zenoh_push_body.DecodeCodecZenohPushBody(cursor)
		if err != nil {
			return nil, err
		}
		_tag := uint8((Header >> 0) & 0x1F)
		body = MakeCodecZenohPushVariantDefault(CodecZenohPushUnknownTag{tag: _tag, set: true}, _arm)
	}
	return &CodecZenohPush{
		header: Header,
		body: body,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohPush) Mid() uint8 {
	return uint8((s.header >> 0) & 0x1F)
}

func (s *CodecZenohPush) SetMid(v uint8) {
	const _shiftedMask uint8 = 0x1F << 0
	_val := (uint8(v) & 0x1F) << 0
	s.header = (s.header &^ _shiftedMask) | _val
}

func (s *CodecZenohPush) Rest() uint8 {
	return uint8((s.header >> 5) & 0x07)
}

func (s *CodecZenohPush) SetRest(v uint8) {
	const _shiftedMask uint8 = 0x07 << 5
	_val := (uint8(v) & 0x07) << 5
	s.header = (s.header &^ _shiftedMask) | _val
}

// Encode writes the CodecZenohPush into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohPush) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohPush) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 3)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
