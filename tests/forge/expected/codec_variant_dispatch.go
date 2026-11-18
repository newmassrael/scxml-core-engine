// SCE-MAP: codec_variant_dispatch:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_variant_dispatch

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_variant_session_open"
	"example.com/sce-forge/codec_variant_session_close"
)

// CodecVariantDispatchUnknownTag is the tag the catch-all arm of CodecVariantDispatch's variant holds: one
// the bits the tag is read from hold and no arm names, held to that where a
// value of it is made (docs/adr/0065 §6 R3, docs/adr/0081 §2), so the catch-all
// a value holds is written and read back as the catch-all. Its zero value
// holds 0x0, the least such tag.
type CodecVariantDispatchUnknownTag struct {
	tag uint8
	set bool
}

// MakeCodecVariantDispatchUnknownTag returns tag as the tag of the catch-all, and false where an
// arm names it or the bits the tag is read from do not hold it.
func MakeCodecVariantDispatchUnknownTag(tag uint8) (CodecVariantDispatchUnknownTag, bool) {
	if tag >= 0x1 && tag <= 0x2 {
		return CodecVariantDispatchUnknownTag{}, false
	}
	return CodecVariantDispatchUnknownTag{tag: tag, set: true}, true
}

// Value returns the tag.
func (t CodecVariantDispatchUnknownTag) Value() uint8 {
	if !t.set {
		return 0x0
	}
	return t.tag
}

// CodecVariantDispatchVariant is the arm of the variant a CodecVariantDispatch holds (docs/adr/0081 §2):
// one of CodecVariantDispatchVariantCodecVariantSessionOpen, CodecVariantDispatchVariantCodecVariantSessionClose, CodecVariantDispatchVariantDefault, which only this package declares.
type CodecVariantDispatchVariant interface {
	isCodecVariantDispatchVariant()
}

// CodecVariantDispatchVariantCodecVariantSessionOpen is the arm of CodecVariantDispatchVariant whose body is a CodecVariantSessionOpen, chosen by the tag 0x1.
type CodecVariantDispatchVariantCodecVariantSessionOpen struct {
	body codec_variant_session_open.CodecVariantSessionOpenSource
}

// MakeCodecVariantDispatchVariantCodecVariantSessionOpen returns the arm that holds what it is handed.
func MakeCodecVariantDispatchVariantCodecVariantSessionOpen(body codec_variant_session_open.CodecVariantSessionOpenSource) CodecVariantDispatchVariantCodecVariantSessionOpen {
	return CodecVariantDispatchVariantCodecVariantSessionOpen{body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecVariantSessionOpen its codec makes by default.
func (a CodecVariantDispatchVariantCodecVariantSessionOpen) Body() codec_variant_session_open.CodecVariantSessionOpenSource {
	if a.body == nil {
		return &codec_variant_session_open.CodecVariantSessionOpen{}
	}
	return a.body
}

func (CodecVariantDispatchVariantCodecVariantSessionOpen) isCodecVariantDispatchVariant() {}

// CodecVariantDispatchVariantCodecVariantSessionClose is the arm of CodecVariantDispatchVariant whose body is a CodecVariantSessionClose, chosen by the tag 0x2.
type CodecVariantDispatchVariantCodecVariantSessionClose struct {
	body codec_variant_session_close.CodecVariantSessionCloseSource
}

// MakeCodecVariantDispatchVariantCodecVariantSessionClose returns the arm that holds what it is handed.
func MakeCodecVariantDispatchVariantCodecVariantSessionClose(body codec_variant_session_close.CodecVariantSessionCloseSource) CodecVariantDispatchVariantCodecVariantSessionClose {
	return CodecVariantDispatchVariantCodecVariantSessionClose{body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecVariantSessionClose its codec makes by default.
func (a CodecVariantDispatchVariantCodecVariantSessionClose) Body() codec_variant_session_close.CodecVariantSessionCloseSource {
	if a.body == nil {
		return &codec_variant_session_close.CodecVariantSessionClose{}
	}
	return a.body
}

func (CodecVariantDispatchVariantCodecVariantSessionClose) isCodecVariantDispatchVariant() {}

// CodecVariantDispatchVariantDefault is the arm of CodecVariantDispatchVariant whose body is a CodecVariantSessionClose, chosen by every tag no arm names.
type CodecVariantDispatchVariantDefault struct {
	tag  CodecVariantDispatchUnknownTag
	body codec_variant_session_close.CodecVariantSessionCloseSource
}

// MakeCodecVariantDispatchVariantDefault returns the arm that holds what it is handed.
func MakeCodecVariantDispatchVariantDefault(tag CodecVariantDispatchUnknownTag, body codec_variant_session_close.CodecVariantSessionCloseSource) CodecVariantDispatchVariantDefault {
	return CodecVariantDispatchVariantDefault{tag: tag, body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecVariantSessionClose its codec makes by default.
func (a CodecVariantDispatchVariantDefault) Body() codec_variant_session_close.CodecVariantSessionCloseSource {
	if a.body == nil {
		return &codec_variant_session_close.CodecVariantSessionClose{}
	}
	return a.body
}

// Tag returns the tag the arm holds, which no arm names.
func (a CodecVariantDispatchVariantDefault) Tag() CodecVariantDispatchUnknownTag {
	return a.tag
}

func (CodecVariantDispatchVariantDefault) isCodecVariantDispatchVariant() {}

// CodecVariantDispatchSource is what the encode of CodecVariantDispatch reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecVariantDispatch is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecVariantDispatchSource interface {
	Body() CodecVariantDispatchVariant
}

// CodecVariantDispatch represents the codec frame layout.
type CodecVariantDispatch struct {
	body CodecVariantDispatchVariant
}

// MakeCodecVariantDispatch returns the CodecVariantDispatch that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecVariantDispatch, or as its zero value, and is read through its methods.
func MakeCodecVariantDispatch(body CodecVariantDispatchVariant) CodecVariantDispatch {
	return CodecVariantDispatch{body: body}
}

// Body returns the arm of the variant the value holds; a value that
// was handed none, as the zero value, holds CodecVariantDispatchVariantCodecVariantSessionClose{}.
func (s *CodecVariantDispatch) Body() CodecVariantDispatchVariant {
	if s.body == nil {
		return CodecVariantDispatchVariantCodecVariantSessionClose{}
	}
	return s.body
}

// WireMsgId returns the msg_id the frame of the value carries: the tag of the
// arm of its variant it holds (docs/adr/0081 §3).
func (s *CodecVariantDispatch) WireMsgId() uint8 {
	var _tBody uint64
	switch _b := s.Body().(type) {
	case CodecVariantDispatchVariantCodecVariantSessionOpen:
		_tBody = 0x1
	case CodecVariantDispatchVariantCodecVariantSessionClose:
		_tBody = 0x2
	case CodecVariantDispatchVariantDefault:
		_tBody = uint64(_b.Tag().Value())
	}
	return uint8(_tBody)
}

// EncodeSource writes any source of CodecVariantDispatch into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives msg_id from
// the arm of the variant the source presents. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecVariantDispatchSource, w codec.SceSink) error {
	_rBody := src.Body()
	var _tBody uint64
	switch _b := _rBody.(type) {
	case CodecVariantDispatchVariantCodecVariantSessionOpen:
		_tBody = 0x1
	case CodecVariantDispatchVariantCodecVariantSessionClose:
		_tBody = 0x2
	case CodecVariantDispatchVariantDefault:
		_tBody = uint64(_b.Tag().Value())
	}
	_dMsgId := uint8(_tBody)
	if err := w.WriteBytes([]byte{byte(_dMsgId)}); err != nil {
		return err
	}
	switch _b := _rBody.(type) {
	case CodecVariantDispatchVariantCodecVariantSessionOpen:
		if err := codec_variant_session_open.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	case CodecVariantDispatchVariantCodecVariantSessionClose:
		if err := codec_variant_session_close.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	case CodecVariantDispatchVariantDefault:
		if err := codec_variant_session_close.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	default:
		panic("CodecVariantDispatch: the source presents no arm of its variant")
	}
	return nil
}

// NewCodecVariantDispatch returns a CodecVariantDispatch initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecVariantDispatch().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecVariantDispatch{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecVariantDispatch() *CodecVariantDispatch {
	return &CodecVariantDispatch{
	}
}

// DecodeCodecVariantDispatch decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecVariantDispatch(cursor *codec.SceCursor) (*CodecVariantDispatch, error) {
	// Decode fixed prefix (RFC §synth-5-B variant: fields before tag suffix).
	raw, err := cursor.PeekSlice(1)
	if err != nil {
		return nil, err
	}
	MsgId := raw[0]
	if err := cursor.Advance(1); err != nil {
		return nil, err
	}
	// Dispatch on the tag field; each arm decodes its body codec from
	// the cursor. The default arm (when declared) carries the runtime
	// tag value so encode can round-trip it back onto the wire.
	var body CodecVariantDispatchVariant
	switch MsgId {
	case 1:
		_arm, err := codec_variant_session_open.DecodeCodecVariantSessionOpen(cursor)
		if err != nil {
			return nil, err
		}
		body = MakeCodecVariantDispatchVariantCodecVariantSessionOpen(_arm)
	case 2:
		_arm, err := codec_variant_session_close.DecodeCodecVariantSessionClose(cursor)
		if err != nil {
			return nil, err
		}
		body = MakeCodecVariantDispatchVariantCodecVariantSessionClose(_arm)
	default:
		_arm, err := codec_variant_session_close.DecodeCodecVariantSessionClose(cursor)
		if err != nil {
			return nil, err
		}
		_tag := MsgId
		body = MakeCodecVariantDispatchVariantDefault(CodecVariantDispatchUnknownTag{tag: _tag, set: true}, _arm)
	}
	return &CodecVariantDispatch{
		body: body,
	}, nil
}

// Encode writes the CodecVariantDispatch into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecVariantDispatch) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecVariantDispatch) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 3)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
