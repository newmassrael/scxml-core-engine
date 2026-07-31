// SCE-MAP: codec_flags_basic:7

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_flags_basic

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecFlagsBasic represents the codec frame layout.
type CodecFlagsBasic struct {
	Header uint8
}

// DecodeCodecFlagsBasic decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecFlagsBasic(cursor *codec.SceCursor) (*CodecFlagsBasic, error) {
	raw, err := cursor.PeekSlice(1)
	if err != nil {
		return nil, err
	}
	Header := raw[0]
	value := &CodecFlagsBasic{
		Header: Header,
	}
	if err := cursor.Advance(1); err != nil {
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
func (s *CodecFlagsBasic) Reliable() bool {
	return (s.Header & 0x80) != 0
}

func (s *CodecFlagsBasic) SetReliable(v bool) {
	if v {
		s.Header |= 0x80
	} else {
		s.Header &^= 0x80
	}
}

func (s *CodecFlagsBasic) More() bool {
	return (s.Header & 0x40) != 0
}

func (s *CodecFlagsBasic) SetMore(v bool) {
	if v {
		s.Header |= 0x40
	} else {
		s.Header &^= 0x40
	}
}

func (s *CodecFlagsBasic) Drop() bool {
	return (s.Header & 0x20) != 0
}

func (s *CodecFlagsBasic) SetDrop(v bool) {
	if v {
		s.Header |= 0x20
	} else {
		s.Header &^= 0x20
	}
}

func (s *CodecFlagsBasic) First() bool {
	return (s.Header & 0x10) != 0
}

func (s *CodecFlagsBasic) SetFirst(v bool) {
	if v {
		s.Header |= 0x10
	} else {
		s.Header &^= 0x10
	}
}

// Encode writes the CodecFlagsBasic into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecFlagsBasic) Encode(w codec.SceSink) error {
	if err := w.WriteBytes([]byte{ byte(s.Header) }); err != nil {
		return err
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecFlagsBasic) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 1)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
