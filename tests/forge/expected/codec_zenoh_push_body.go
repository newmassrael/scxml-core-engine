// SCE-MAP: codec_zenoh_push_body:30 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_push_body

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_put"
	"example.com/sce-forge/codec_zenoh_del"
)

// CodecZenohPushBodyUnknownTag is the tag the catch-all arm of CodecZenohPushBody's variant holds: one
// the bits the tag is read from hold and no arm names, held to that where a
// value of it is made (docs/adr/0065 §6 R3, docs/adr/0081 §2), so the catch-all
// a value holds is written and read back as the catch-all. Its zero value
// holds 0x0, the least such tag.
type CodecZenohPushBodyUnknownTag struct {
	tag uint8
	set bool
}

// MakeCodecZenohPushBodyUnknownTag returns tag as the tag of the catch-all, and false where an
// arm names it or the bits the tag is read from do not hold it.
func MakeCodecZenohPushBodyUnknownTag(tag uint8) (CodecZenohPushBodyUnknownTag, bool) {
	if tag >= 0x1 && tag <= 0x2 || tag > 0x1F {
		return CodecZenohPushBodyUnknownTag{}, false
	}
	return CodecZenohPushBodyUnknownTag{tag: tag, set: true}, true
}

// Value returns the tag.
func (t CodecZenohPushBodyUnknownTag) Value() uint8 {
	if !t.set {
		return 0x0
	}
	return t.tag
}

// CodecZenohPushBodyVariant is the arm of the variant a CodecZenohPushBody holds (docs/adr/0081 §2):
// one of CodecZenohPushBodyVariantCodecZenohPut, CodecZenohPushBodyVariantCodecZenohDel, CodecZenohPushBodyVariantDefault, which only this package declares.
type CodecZenohPushBodyVariant interface {
	isCodecZenohPushBodyVariant()
}

// CodecZenohPushBodyVariantCodecZenohPut is the arm of CodecZenohPushBodyVariant whose body is a CodecZenohPut, chosen by the tag 0x1.
type CodecZenohPushBodyVariantCodecZenohPut struct {
	body codec_zenoh_put.CodecZenohPutSource
}

// MakeCodecZenohPushBodyVariantCodecZenohPut returns the arm that holds what it is handed.
func MakeCodecZenohPushBodyVariantCodecZenohPut(body codec_zenoh_put.CodecZenohPutSource) CodecZenohPushBodyVariantCodecZenohPut {
	return CodecZenohPushBodyVariantCodecZenohPut{body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecZenohPut its codec makes by default.
func (a CodecZenohPushBodyVariantCodecZenohPut) Body() codec_zenoh_put.CodecZenohPutSource {
	if a.body == nil {
		return &codec_zenoh_put.CodecZenohPut{}
	}
	return a.body
}

func (CodecZenohPushBodyVariantCodecZenohPut) isCodecZenohPushBodyVariant() {}

// CodecZenohPushBodyVariantCodecZenohDel is the arm of CodecZenohPushBodyVariant whose body is a CodecZenohDel, chosen by the tag 0x2.
type CodecZenohPushBodyVariantCodecZenohDel struct {
	body codec_zenoh_del.CodecZenohDelSource
}

// MakeCodecZenohPushBodyVariantCodecZenohDel returns the arm that holds what it is handed.
func MakeCodecZenohPushBodyVariantCodecZenohDel(body codec_zenoh_del.CodecZenohDelSource) CodecZenohPushBodyVariantCodecZenohDel {
	return CodecZenohPushBodyVariantCodecZenohDel{body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecZenohDel its codec makes by default.
func (a CodecZenohPushBodyVariantCodecZenohDel) Body() codec_zenoh_del.CodecZenohDelSource {
	if a.body == nil {
		return &codec_zenoh_del.CodecZenohDel{}
	}
	return a.body
}

func (CodecZenohPushBodyVariantCodecZenohDel) isCodecZenohPushBodyVariant() {}

// CodecZenohPushBodyVariantDefault is the arm of CodecZenohPushBodyVariant whose body is a CodecZenohPut, chosen by every tag no arm names.
type CodecZenohPushBodyVariantDefault struct {
	tag  CodecZenohPushBodyUnknownTag
	body codec_zenoh_put.CodecZenohPutSource
}

// MakeCodecZenohPushBodyVariantDefault returns the arm that holds what it is handed.
func MakeCodecZenohPushBodyVariantDefault(tag CodecZenohPushBodyUnknownTag, body codec_zenoh_put.CodecZenohPutSource) CodecZenohPushBodyVariantDefault {
	return CodecZenohPushBodyVariantDefault{tag: tag, body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecZenohPut its codec makes by default.
func (a CodecZenohPushBodyVariantDefault) Body() codec_zenoh_put.CodecZenohPutSource {
	if a.body == nil {
		return &codec_zenoh_put.CodecZenohPut{}
	}
	return a.body
}

// Tag returns the tag the arm holds, which no arm names.
func (a CodecZenohPushBodyVariantDefault) Tag() CodecZenohPushBodyUnknownTag {
	return a.tag
}

func (CodecZenohPushBodyVariantDefault) isCodecZenohPushBodyVariant() {}

// CodecZenohPushBodySource is what the encode of CodecZenohPushBody reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohPushBody is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohPushBodySource interface {
	Header() uint8
	Body() CodecZenohPushBodyVariant
}

// CodecZenohPushBody represents the codec frame layout.
type CodecZenohPushBody struct {
	header uint8
	body CodecZenohPushBodyVariant
}

// MakeCodecZenohPushBody returns the CodecZenohPushBody that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohPushBody, or as its zero value, and is read through its methods.
func MakeCodecZenohPushBody(header uint8, body CodecZenohPushBodyVariant) CodecZenohPushBody {
	return CodecZenohPushBody{header: header, body: body}
}

// Header returns the element the value presents as Header.
func (s *CodecZenohPushBody) Header() uint8 {
	return s.header
}

// Body returns the arm of the variant the value holds; a value that
// was handed none, as the zero value, holds CodecZenohPushBodyVariantCodecZenohPut{}.
func (s *CodecZenohPushBody) Body() CodecZenohPushBodyVariant {
	if s.body == nil {
		return CodecZenohPushBodyVariantCodecZenohPut{}
	}
	return s.body
}

// EncodeSource writes any source of CodecZenohPushBody into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohPushBodySource, w codec.SceSink) error {
	_rBody := src.Body()
	var _tBody uint64
	switch _b := _rBody.(type) {
	case CodecZenohPushBodyVariantCodecZenohPut:
		_tBody = 0x1
	case CodecZenohPushBodyVariantCodecZenohDel:
		_tBody = 0x2
	case CodecZenohPushBodyVariantDefault:
		_tBody = uint64(_b.Tag().Value())
	}
	_cHeader := src.Header()
	_cHeader = (_cHeader &^ 0x1F) | (uint8(_tBody & 0x1F) << 0)
	if err := w.WriteBytes([]byte{byte(_cHeader)}); err != nil {
		return err
	}
	switch _b := _rBody.(type) {
	case CodecZenohPushBodyVariantCodecZenohPut:
		if err := codec_zenoh_put.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	case CodecZenohPushBodyVariantCodecZenohDel:
		if err := codec_zenoh_del.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	case CodecZenohPushBodyVariantDefault:
		if err := codec_zenoh_put.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	default:
		panic("CodecZenohPushBody: the source presents no arm of its variant")
	}
	return nil
}

// NewCodecZenohPushBody returns a CodecZenohPushBody initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecZenohPushBody().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecZenohPushBody{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecZenohPushBody() *CodecZenohPushBody {
	return &CodecZenohPushBody{
	}
}

// DecodeCodecZenohPushBody decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohPushBody(cursor *codec.SceCursor) (*CodecZenohPushBody, error) {
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
	var body CodecZenohPushBodyVariant
	switch uint8((Header >> 0) & 0x1F) {
	case 1:
		_arm, err := codec_zenoh_put.DecodeCodecZenohPut(cursor)
		if err != nil {
			return nil, err
		}
		body = MakeCodecZenohPushBodyVariantCodecZenohPut(_arm)
	case 2:
		_arm, err := codec_zenoh_del.DecodeCodecZenohDel(cursor)
		if err != nil {
			return nil, err
		}
		body = MakeCodecZenohPushBodyVariantCodecZenohDel(_arm)
	default:
		_arm, err := codec_zenoh_put.DecodeCodecZenohPut(cursor)
		if err != nil {
			return nil, err
		}
		_tag := uint8((Header >> 0) & 0x1F)
		body = MakeCodecZenohPushBodyVariantDefault(CodecZenohPushBodyUnknownTag{tag: _tag, set: true}, _arm)
	}
	return &CodecZenohPushBody{
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
func (s *CodecZenohPushBody) Mid() uint8 {
	return uint8((s.header >> 0) & 0x1F)
}

func (s *CodecZenohPushBody) SetMid(v uint8) {
	const _shiftedMask uint8 = 0x1F << 0
	_val := (uint8(v) & 0x1F) << 0
	s.header = (s.header &^ _shiftedMask) | _val
}

func (s *CodecZenohPushBody) Rest() uint8 {
	return uint8((s.header >> 5) & 0x07)
}

func (s *CodecZenohPushBody) SetRest(v uint8) {
	const _shiftedMask uint8 = 0x07 << 5
	_val := (uint8(v) & 0x07) << 5
	s.header = (s.header &^ _shiftedMask) | _val
}

// Encode writes the CodecZenohPushBody into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohPushBody) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohPushBody) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 2)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
