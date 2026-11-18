// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_variant_default_marker.scxml:30 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_variant_default_marker

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/round_trip/generated/codec_default_marker_arm_a"
	"github.com/newmassrael/sce-forge-runtime/round_trip/generated/codec_default_marker_arm_b"
)

// CodecVariantDefaultMarkerUnknownTag is the tag the catch-all arm of CodecVariantDefaultMarker's variant holds: one
// the bits the tag is read from hold and no arm names, held to that where a
// value of it is made (docs/adr/0065 §6 R3, docs/adr/0081 §2), so the catch-all
// a value holds is written and read back as the catch-all. Its zero value
// holds 0x0, the least such tag.
type CodecVariantDefaultMarkerUnknownTag struct {
	tag uint8
	set bool
}

// MakeCodecVariantDefaultMarkerUnknownTag returns tag as the tag of the catch-all, and false where an
// arm names it or the bits the tag is read from do not hold it.
func MakeCodecVariantDefaultMarkerUnknownTag(tag uint8) (CodecVariantDefaultMarkerUnknownTag, bool) {
	if tag >= 0x1 && tag <= 0x2 || tag > 0x3 {
		return CodecVariantDefaultMarkerUnknownTag{}, false
	}
	return CodecVariantDefaultMarkerUnknownTag{tag: tag, set: true}, true
}

// Value returns the tag.
func (t CodecVariantDefaultMarkerUnknownTag) Value() uint8 {
	if !t.set {
		return 0x0
	}
	return t.tag
}

// CodecVariantDefaultMarkerVariant is the arm of the variant a CodecVariantDefaultMarker holds (docs/adr/0081 §2):
// one of CodecVariantDefaultMarkerVariantCodecDefaultMarkerArmA, CodecVariantDefaultMarkerVariantCodecDefaultMarkerArmB, CodecVariantDefaultMarkerVariantDefault, which only this package declares.
type CodecVariantDefaultMarkerVariant interface {
	isCodecVariantDefaultMarkerVariant()
}

// CodecVariantDefaultMarkerVariantCodecDefaultMarkerArmA is the arm of CodecVariantDefaultMarkerVariant whose body is a CodecDefaultMarkerArmA, chosen by the tag 0x1.
type CodecVariantDefaultMarkerVariantCodecDefaultMarkerArmA struct {
	body codec_default_marker_arm_a.CodecDefaultMarkerArmASource
}

// MakeCodecVariantDefaultMarkerVariantCodecDefaultMarkerArmA returns the arm that holds what it is handed.
func MakeCodecVariantDefaultMarkerVariantCodecDefaultMarkerArmA(body codec_default_marker_arm_a.CodecDefaultMarkerArmASource) CodecVariantDefaultMarkerVariantCodecDefaultMarkerArmA {
	return CodecVariantDefaultMarkerVariantCodecDefaultMarkerArmA{body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecDefaultMarkerArmA its codec makes by default.
func (a CodecVariantDefaultMarkerVariantCodecDefaultMarkerArmA) Body() codec_default_marker_arm_a.CodecDefaultMarkerArmASource {
	if a.body == nil {
		return codec_default_marker_arm_a.NewCodecDefaultMarkerArmA()
	}
	return a.body
}

func (CodecVariantDefaultMarkerVariantCodecDefaultMarkerArmA) isCodecVariantDefaultMarkerVariant() {}

// CodecVariantDefaultMarkerVariantCodecDefaultMarkerArmB is the arm of CodecVariantDefaultMarkerVariant whose body is a CodecDefaultMarkerArmB, chosen by the tag 0x2.
type CodecVariantDefaultMarkerVariantCodecDefaultMarkerArmB struct {
	body codec_default_marker_arm_b.CodecDefaultMarkerArmBSource
}

// MakeCodecVariantDefaultMarkerVariantCodecDefaultMarkerArmB returns the arm that holds what it is handed.
func MakeCodecVariantDefaultMarkerVariantCodecDefaultMarkerArmB(body codec_default_marker_arm_b.CodecDefaultMarkerArmBSource) CodecVariantDefaultMarkerVariantCodecDefaultMarkerArmB {
	return CodecVariantDefaultMarkerVariantCodecDefaultMarkerArmB{body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecDefaultMarkerArmB its codec makes by default.
func (a CodecVariantDefaultMarkerVariantCodecDefaultMarkerArmB) Body() codec_default_marker_arm_b.CodecDefaultMarkerArmBSource {
	if a.body == nil {
		return codec_default_marker_arm_b.NewCodecDefaultMarkerArmB()
	}
	return a.body
}

func (CodecVariantDefaultMarkerVariantCodecDefaultMarkerArmB) isCodecVariantDefaultMarkerVariant() {}

// CodecVariantDefaultMarkerVariantDefault is the arm of CodecVariantDefaultMarkerVariant whose body is a CodecDefaultMarkerArmB, chosen by every tag no arm names.
type CodecVariantDefaultMarkerVariantDefault struct {
	tag  CodecVariantDefaultMarkerUnknownTag
	body codec_default_marker_arm_b.CodecDefaultMarkerArmBSource
}

// MakeCodecVariantDefaultMarkerVariantDefault returns the arm that holds what it is handed.
func MakeCodecVariantDefaultMarkerVariantDefault(tag CodecVariantDefaultMarkerUnknownTag, body codec_default_marker_arm_b.CodecDefaultMarkerArmBSource) CodecVariantDefaultMarkerVariantDefault {
	return CodecVariantDefaultMarkerVariantDefault{tag: tag, body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecDefaultMarkerArmB its codec makes by default.
func (a CodecVariantDefaultMarkerVariantDefault) Body() codec_default_marker_arm_b.CodecDefaultMarkerArmBSource {
	if a.body == nil {
		return codec_default_marker_arm_b.NewCodecDefaultMarkerArmB()
	}
	return a.body
}

// Tag returns the tag the arm holds, which no arm names.
func (a CodecVariantDefaultMarkerVariantDefault) Tag() CodecVariantDefaultMarkerUnknownTag {
	return a.tag
}

func (CodecVariantDefaultMarkerVariantDefault) isCodecVariantDefaultMarkerVariant() {}

// CodecVariantDefaultMarkerSource is what the encode of CodecVariantDefaultMarker reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecVariantDefaultMarker is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecVariantDefaultMarkerSource interface {
	Body() CodecVariantDefaultMarkerVariant
}

// CodecVariantDefaultMarker represents the codec frame layout.
type CodecVariantDefaultMarker struct {
	body CodecVariantDefaultMarkerVariant
}

// MakeCodecVariantDefaultMarker returns the CodecVariantDefaultMarker that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecVariantDefaultMarker, or as its zero value, and is read through its methods.
func MakeCodecVariantDefaultMarker(body CodecVariantDefaultMarkerVariant) CodecVariantDefaultMarker {
	return CodecVariantDefaultMarker{body: body}
}

// Body returns the arm of the variant the value holds; a value that
// was handed none, as the zero value, holds CodecVariantDefaultMarkerVariantCodecDefaultMarkerArmB{}.
func (s *CodecVariantDefaultMarker) Body() CodecVariantDefaultMarkerVariant {
	if s.body == nil {
		return CodecVariantDefaultMarkerVariantCodecDefaultMarkerArmB{}
	}
	return s.body
}

// EncodeSource writes any source of CodecVariantDefaultMarker into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecVariantDefaultMarkerSource, w codec.SceSink) error {
	_rBody := src.Body()
	switch _b := _rBody.(type) {
	case CodecVariantDefaultMarkerVariantCodecDefaultMarkerArmA:
		if err := codec_default_marker_arm_a.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	case CodecVariantDefaultMarkerVariantCodecDefaultMarkerArmB:
		if err := codec_default_marker_arm_b.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	case CodecVariantDefaultMarkerVariantDefault:
		if err := codec_default_marker_arm_b.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	default:
		panic("CodecVariantDefaultMarker: the source presents no arm of its variant")
	}
	return nil
}

// NewCodecVariantDefaultMarker returns a CodecVariantDefaultMarker initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecVariantDefaultMarker().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecVariantDefaultMarker{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecVariantDefaultMarker() *CodecVariantDefaultMarker {
	return &CodecVariantDefaultMarker{
	}
}

// DecodeCodecVariantDefaultMarker decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecVariantDefaultMarker(cursor *codec.SceCursor) (*CodecVariantDefaultMarker, error) {
	// RFC §synth-5-B peek-byte / streaming-prefix:
	// streaming prefix decode (variable-length fields supported via
	// per-field present_if/tlv-chain/embed/repeat helpers). Peek-byte
	// mode additionally peeks the cursor's next byte for variant tag
	// without advancing — arm body decoder reads it as own header.
	_peekSlice, err := cursor.PeekSlice(1)
	if err != nil {
		return nil, err
	}
	_peek := _peekSlice[0]
	// Dispatch on the tag field; each arm decodes its body codec from
	// the cursor. The default arm (when declared) carries the runtime
	// tag value so encode can round-trip it back onto the wire.
	var body CodecVariantDefaultMarkerVariant
	switch uint8((_peek >> 0) & 0x03) {
	case 1:
		_arm, err := codec_default_marker_arm_a.DecodeCodecDefaultMarkerArmA(cursor)
		if err != nil {
			return nil, err
		}
		body = MakeCodecVariantDefaultMarkerVariantCodecDefaultMarkerArmA(_arm)
	case 2:
		_arm, err := codec_default_marker_arm_b.DecodeCodecDefaultMarkerArmB(cursor)
		if err != nil {
			return nil, err
		}
		body = MakeCodecVariantDefaultMarkerVariantCodecDefaultMarkerArmB(_arm)
	default:
		_arm, err := codec_default_marker_arm_b.DecodeCodecDefaultMarkerArmB(cursor)
		if err != nil {
			return nil, err
		}
		_tag := uint8((_peek >> 0) & 0x03)
		body = MakeCodecVariantDefaultMarkerVariantDefault(CodecVariantDefaultMarkerUnknownTag{tag: _tag, set: true}, _arm)
	}
	return &CodecVariantDefaultMarker{
		body: body,
	}, nil
}

// Encode writes the CodecVariantDefaultMarker into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecVariantDefaultMarker) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecVariantDefaultMarker) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 3)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
