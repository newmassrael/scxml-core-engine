// SCE-MAP: codec_peek_arm_b:13

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_peek_arm_b

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecPeekArmB represents the codec frame layout.
type CodecPeekArmB struct {
	Header uint8
	Payload uint16
}

// NewCodecPeekArmB returns a CodecPeekArmB initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecPeekArmB().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecPeekArmB{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecPeekArmB() *CodecPeekArmB {
	return &CodecPeekArmB{
		Header: uint8(0x01),
	}
}

// DecodeCodecPeekArmB decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecPeekArmB(cursor *codec.SceCursor) (*CodecPeekArmB, error) {
	raw, err := cursor.PeekSlice(3)
	if err != nil {
		return nil, err
	}
	Header := raw[0]
	Payload := uint16(raw[1])<<8 | uint16(raw[2])
	value := &CodecPeekArmB{
		Header: Header,
		Payload: Payload,
	}
	if err := cursor.Advance(3); err != nil {
		return nil, err
	}
	return value, nil
}

// RFC §5.B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecPeekArmB) Kind() bool {
	return (s.Header & 0x01) != 0
}

func (s *CodecPeekArmB) SetKind(v bool) {
	if v {
		s.Header |= 0x01
	} else {
		s.Header &^= 0x01
	}
}

// Encode writes the CodecPeekArmB into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecPeekArmB) Encode(w codec.SceSink) error {
	if err := w.WriteBytes([]byte{ byte(s.Header) }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ byte(s.Payload >> 8 & 0xFF) }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ byte(s.Payload & 0xFF) }); err != nil {
		return err
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecPeekArmB) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 3)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
