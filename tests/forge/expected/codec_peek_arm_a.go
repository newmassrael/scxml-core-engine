// SCE-MAP: codec_peek_arm_a:13

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_peek_arm_a

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecPeekArmA represents the codec frame layout.
type CodecPeekArmA struct {
	Header uint8
	Payload uint8
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
		Header: uint8(0x00),
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
		Header: Header,
		Payload: Payload,
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
	return (s.Header & 0x01) != 0
}

func (s *CodecPeekArmA) SetKind(v bool) {
	if v {
		s.Header |= 0x01
	} else {
		s.Header &^= 0x01
	}
}

// Encode writes the CodecPeekArmA into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecPeekArmA) Encode(w codec.SceSink) error {
	if err := w.WriteBytes([]byte{ byte(s.Header) }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ byte(s.Payload) }); err != nil {
		return err
	}
	return nil
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
