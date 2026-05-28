// SCE-GENERATED — DO NOT EDIT
// source-hash: ada27af98349515297e32a6009fc338413bcbcccb7886d535e7ca5bb13a54071
// template-hash: 30c21c2126baf025b95abcba3754b0bcf0280066f6d16a0568643a49c1942e1f
// generated-at: 1779967212
// SCE-MAP: codec_default_marker_arm_a.scxml:16

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_default_marker_arm_a

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecDefaultMarkerArmA represents the codec frame layout.
type CodecDefaultMarkerArmA struct {
	Header uint8
	Payload uint8
}

// NewCodecDefaultMarkerArmA returns a CodecDefaultMarkerArmA initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecDefaultMarkerArmA().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecDefaultMarkerArmA{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity Atomic β-go.
func NewCodecDefaultMarkerArmA() *CodecDefaultMarkerArmA {
	return &CodecDefaultMarkerArmA{
		Header: uint8(0x01),
	}
}

// DecodeCodecDefaultMarkerArmA decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecDefaultMarkerArmA(cursor *codec.SceCursor) (*CodecDefaultMarkerArmA, error) {
	raw, err := cursor.PeekSlice(2)
	if err != nil {
		return nil, err
	}
	Header := raw[0]
	Payload := raw[1]
	value := &CodecDefaultMarkerArmA{
		Header: Header,
		Payload: Payload,
	}
	if err := cursor.Advance(2); err != nil {
		return nil, err
	}
	return value, nil
}

// RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecDefaultMarkerArmA) Kind() uint8 {
	return uint8((s.Header >> 0) & 0x03)
}

func (s *CodecDefaultMarkerArmA) SetKind(v uint8) {
	const _shiftedMask uint8 = 0x03 << 0
	_val := (uint8(v) & 0x03) << 0
	s.Header = (s.Header &^ _shiftedMask) | _val
}

// Encode writes the CodecDefaultMarkerArmA into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecDefaultMarkerArmA) Encode(w codec.SceSink) error {
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
func (s *CodecDefaultMarkerArmA) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 2)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}