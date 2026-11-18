// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e14e61a19815f19d3d0dbb6fe1326691e48aee7d4809e1595ab8383fb1a9fb7
// SCE-MAP: codec_default_marker_arm_a.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_default_marker_arm_a

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecDefaultMarkerArmASource is what the encode of CodecDefaultMarkerArmA reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecDefaultMarkerArmA is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecDefaultMarkerArmASource interface {
	Header() uint8
	Payload() uint8
}

// CodecDefaultMarkerArmA represents the codec frame layout.
type CodecDefaultMarkerArmA struct {
	header uint8
	payload uint8
}

// MakeCodecDefaultMarkerArmA returns the CodecDefaultMarkerArmA that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecDefaultMarkerArmA, or as its zero value, and is read through its methods.
func MakeCodecDefaultMarkerArmA(header uint8, payload uint8) CodecDefaultMarkerArmA {
	return CodecDefaultMarkerArmA{header: header, payload: payload}
}

// Header returns the element the value presents as Header.
func (s *CodecDefaultMarkerArmA) Header() uint8 {
	return s.header
}

// Payload returns the element the value presents as Payload.
func (s *CodecDefaultMarkerArmA) Payload() uint8 {
	return s.payload
}

// EncodeSource writes any source of CodecDefaultMarkerArmA into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecDefaultMarkerArmASource, w codec.SceSink) error {
	if err := w.WriteBytes([]byte{byte(src.Header())}); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{byte(src.Payload())}); err != nil {
		return err
	}
	return nil
}

// NewCodecDefaultMarkerArmA returns a CodecDefaultMarkerArmA initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecDefaultMarkerArmA().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecDefaultMarkerArmA{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecDefaultMarkerArmA() *CodecDefaultMarkerArmA {
	return &CodecDefaultMarkerArmA{
		header: uint8(0x01),
	}
}

// DecodeCodecDefaultMarkerArmA decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecDefaultMarkerArmA(cursor *codec.SceCursor) (*CodecDefaultMarkerArmA, error) {
	raw, err := cursor.PeekSlice(2)
	if err != nil {
		return nil, err
	}
	Header := raw[0]
	Payload := raw[1]
	value := &CodecDefaultMarkerArmA{
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
func (s *CodecDefaultMarkerArmA) Kind() uint8 {
	return uint8((s.header >> 0) & 0x03)
}

func (s *CodecDefaultMarkerArmA) SetKind(v uint8) {
	const _shiftedMask uint8 = 0x03 << 0
	_val := (uint8(v) & 0x03) << 0
	s.header = (s.header &^ _shiftedMask) | _val
}

// Encode writes the CodecDefaultMarkerArmA into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecDefaultMarkerArmA) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecDefaultMarkerArmA) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 2)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
