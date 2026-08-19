// SCE-GENERATED — DO NOT EDIT
// source-hash: 6a06e9c790d7c99ded460925699421079f0734d06570cc7b08b90568979799bd
// template-hash: 08524b6e9f06ec235417da53ac7c80c6bfd4ac29c2f21bcfec9a9e720a464526
// generated-at: 0
// SCE-MAP: codec_default_marker_arm_b.scxml:14 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_default_marker_arm_b

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecDefaultMarkerArmB represents the codec frame layout.
type CodecDefaultMarkerArmB struct {
	Header uint8
	Payload uint16
}

// NewCodecDefaultMarkerArmB returns a CodecDefaultMarkerArmB initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecDefaultMarkerArmB().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecDefaultMarkerArmB{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecDefaultMarkerArmB() *CodecDefaultMarkerArmB {
	return &CodecDefaultMarkerArmB{
		Header: uint8(0x02),
	}
}

// DecodeCodecDefaultMarkerArmB decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecDefaultMarkerArmB(cursor *codec.SceCursor) (*CodecDefaultMarkerArmB, error) {
	raw, err := cursor.PeekSlice(3)
	if err != nil {
		return nil, err
	}
	Header := raw[0]
	Payload := uint16(raw[1])<<8 | uint16(raw[2])
	value := &CodecDefaultMarkerArmB{
		Header: Header,
		Payload: Payload,
	}
	if err := cursor.Advance(3); err != nil {
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
func (s *CodecDefaultMarkerArmB) Kind() uint8 {
	return uint8((s.Header >> 0) & 0x03)
}

func (s *CodecDefaultMarkerArmB) SetKind(v uint8) {
	const _shiftedMask uint8 = 0x03 << 0
	_val := (uint8(v) & 0x03) << 0
	s.Header = (s.Header &^ _shiftedMask) | _val
}

// Encode writes the CodecDefaultMarkerArmB into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecDefaultMarkerArmB) Encode(w codec.SceSink) error {
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
func (s *CodecDefaultMarkerArmB) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 3)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
