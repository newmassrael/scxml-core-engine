// SCE-MAP: codec_variant_peek_basic:29 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_variant_peek_basic

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_peek_arm_a"
	"example.com/sce-forge/codec_peek_arm_b"
)

// CodecVariantPeekBasicVariant is the arm of the variant a CodecVariantPeekBasic holds (docs/adr/0081 §2):
// one of CodecVariantPeekBasicVariantCodecPeekArmA, CodecVariantPeekBasicVariantCodecPeekArmB, which only this package declares.
type CodecVariantPeekBasicVariant interface {
	isCodecVariantPeekBasicVariant()
}

// CodecVariantPeekBasicVariantCodecPeekArmA is the arm of CodecVariantPeekBasicVariant whose body is a CodecPeekArmA, chosen by the tag 0x0.
type CodecVariantPeekBasicVariantCodecPeekArmA struct {
	body codec_peek_arm_a.CodecPeekArmASource
}

// MakeCodecVariantPeekBasicVariantCodecPeekArmA returns the arm that holds what it is handed.
func MakeCodecVariantPeekBasicVariantCodecPeekArmA(body codec_peek_arm_a.CodecPeekArmASource) CodecVariantPeekBasicVariantCodecPeekArmA {
	return CodecVariantPeekBasicVariantCodecPeekArmA{body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecPeekArmA its codec makes by default.
func (a CodecVariantPeekBasicVariantCodecPeekArmA) Body() codec_peek_arm_a.CodecPeekArmASource {
	if a.body == nil {
		return codec_peek_arm_a.NewCodecPeekArmA()
	}
	return a.body
}

func (CodecVariantPeekBasicVariantCodecPeekArmA) isCodecVariantPeekBasicVariant() {}

// CodecVariantPeekBasicVariantCodecPeekArmB is the arm of CodecVariantPeekBasicVariant whose body is a CodecPeekArmB, chosen by the tag 0x1.
type CodecVariantPeekBasicVariantCodecPeekArmB struct {
	body codec_peek_arm_b.CodecPeekArmBSource
}

// MakeCodecVariantPeekBasicVariantCodecPeekArmB returns the arm that holds what it is handed.
func MakeCodecVariantPeekBasicVariantCodecPeekArmB(body codec_peek_arm_b.CodecPeekArmBSource) CodecVariantPeekBasicVariantCodecPeekArmB {
	return CodecVariantPeekBasicVariantCodecPeekArmB{body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecPeekArmB its codec makes by default.
func (a CodecVariantPeekBasicVariantCodecPeekArmB) Body() codec_peek_arm_b.CodecPeekArmBSource {
	if a.body == nil {
		return codec_peek_arm_b.NewCodecPeekArmB()
	}
	return a.body
}

func (CodecVariantPeekBasicVariantCodecPeekArmB) isCodecVariantPeekBasicVariant() {}

// CodecVariantPeekBasicSource is what the encode of CodecVariantPeekBasic reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecVariantPeekBasic is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecVariantPeekBasicSource interface {
	Body() CodecVariantPeekBasicVariant
}

// CodecVariantPeekBasic represents the codec frame layout.
type CodecVariantPeekBasic struct {
	body CodecVariantPeekBasicVariant
}

// MakeCodecVariantPeekBasic returns the CodecVariantPeekBasic that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecVariantPeekBasic, or as its zero value, and is read through its methods.
func MakeCodecVariantPeekBasic(body CodecVariantPeekBasicVariant) CodecVariantPeekBasic {
	return CodecVariantPeekBasic{body: body}
}

// Body returns the arm of the variant the value holds; a value that
// was handed none, as the zero value, holds CodecVariantPeekBasicVariantCodecPeekArmA{}.
func (s *CodecVariantPeekBasic) Body() CodecVariantPeekBasicVariant {
	if s.body == nil {
		return CodecVariantPeekBasicVariantCodecPeekArmA{}
	}
	return s.body
}

// EncodeSource writes any source of CodecVariantPeekBasic into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecVariantPeekBasicSource, w codec.SceSink) error {
	_rBody := src.Body()
	switch _b := _rBody.(type) {
	case CodecVariantPeekBasicVariantCodecPeekArmA:
		if err := codec_peek_arm_a.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	case CodecVariantPeekBasicVariantCodecPeekArmB:
		if err := codec_peek_arm_b.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	default:
		panic("CodecVariantPeekBasic: the source presents no arm of its variant")
	}
	return nil
}

// NewCodecVariantPeekBasic returns a CodecVariantPeekBasic initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecVariantPeekBasic().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecVariantPeekBasic{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecVariantPeekBasic() *CodecVariantPeekBasic {
	return &CodecVariantPeekBasic{
	}
}

// DecodeCodecVariantPeekBasic decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecVariantPeekBasic(cursor *codec.SceCursor) (*CodecVariantPeekBasic, error) {
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
	var body CodecVariantPeekBasicVariant
	switch uint8((_peek >> 0) & 0x01) {
	case 0:
		_arm, err := codec_peek_arm_a.DecodeCodecPeekArmA(cursor)
		if err != nil {
			return nil, err
		}
		body = MakeCodecVariantPeekBasicVariantCodecPeekArmA(_arm)
	case 1:
		_arm, err := codec_peek_arm_b.DecodeCodecPeekArmB(cursor)
		if err != nil {
			return nil, err
		}
		body = MakeCodecVariantPeekBasicVariantCodecPeekArmB(_arm)
	default:
		// codec/variant-arm-unreachable rejected this case at parse time.
		return nil, codec.ErrNeedMoreBytes
	}
	return &CodecVariantPeekBasic{
		body: body,
	}, nil
}

// Encode writes the CodecVariantPeekBasic into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecVariantPeekBasic) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecVariantPeekBasic) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 3)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
