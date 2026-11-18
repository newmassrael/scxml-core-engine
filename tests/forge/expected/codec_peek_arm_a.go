// SCE-MAP: codec_peek_arm_a:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_peek_arm_a

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
)

// CodecPeekArmASource is what the encode of CodecPeekArmA reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecPeekArmA is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecPeekArmASource interface {
	Header() uint8
	Payload() uint8
}

// CodecPeekArmA represents the codec frame layout.
type CodecPeekArmA struct {
	header uint8
	payload uint8
}

// MakeCodecPeekArmA returns the CodecPeekArmA that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecPeekArmA, or as its zero value, and is read through its methods.
func MakeCodecPeekArmA(header uint8, payload uint8) CodecPeekArmA {
	return CodecPeekArmA{header: header, payload: payload}
}

// Header returns the element the value presents as Header.
func (s *CodecPeekArmA) Header() uint8 {
	return s.header
}

// Payload returns the element the value presents as Payload.
func (s *CodecPeekArmA) Payload() uint8 {
	return s.payload
}

// CodecPeekArmAOverrides holds the one-bit flags of CodecPeekArmA a parent may
// set in place of the ones a value presents, such as the flag a chain whose
// entries are CodecPeekArmA ends them on, which the chain derives from where
// each entry stands. A flag it leaves absent is the one the value presents.
type CodecPeekArmAOverrides struct {
	// HeaderKind is the `kind` flag of `header`, where a parent sets it.
	HeaderKind optional.Of[bool]
}

// EncodeSource writes any source of CodecPeekArmA into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecPeekArmASource, w codec.SceSink) error {
	return EncodeSourceOverriding(src, w, CodecPeekArmAOverrides{})
}

// EncodeSourceOverriding is EncodeSource, each flag overrides sets written in
// place of the one src presents.
func EncodeSourceOverriding(src CodecPeekArmASource, w codec.SceSink, overrides CodecPeekArmAOverrides) error {
	_cHeader := src.Header()
	if _o, ok := overrides.HeaderKind.Get(); ok {
		_cHeader &^= 0x1
		if _o {
			_cHeader |= 0x1
		}
	}
	if err := w.WriteBytes([]byte{byte(_cHeader)}); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{byte(src.Payload())}); err != nil {
		return err
	}
	return nil
}

// NewCodecPeekArmA returns a CodecPeekArmA initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecPeekArmA().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecPeekArmA{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecPeekArmA() *CodecPeekArmA {
	return &CodecPeekArmA{
		header: uint8(0x00),
	}
}

// DecodeCodecPeekArmA decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecPeekArmA(cursor *codec.SceCursor) (*CodecPeekArmA, error) {
	raw, err := cursor.PeekSlice(2)
	if err != nil {
		return nil, err
	}
	Header := raw[0]
	Payload := raw[1]
	value := &CodecPeekArmA{
		header: Header,
		payload: Payload,
	}
	if err := cursor.Advance(2); err != nil {
		return nil, err
	}
	return value, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecPeekArmA) Kind() bool {
	return (s.header & 0x01) != 0
}

func (s *CodecPeekArmA) SetKind(v bool) {
	if v {
		s.header |= 0x01
	} else {
		s.header &^= 0x01
	}
}

// Encode writes the CodecPeekArmA into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecPeekArmA) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecPeekArmA) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 2)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
