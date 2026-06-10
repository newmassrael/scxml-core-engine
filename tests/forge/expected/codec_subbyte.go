// SCE-MAP: codec_subbyte:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_subbyte

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecSubbyte represents the codec frame layout.
type CodecSubbyte struct {
	Priority uint8
	Channel uint8
	Direction uint8
}

// DecodeCodecSubbyte decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecSubbyte(cursor *codec.SceCursor) (*CodecSubbyte, error) {
	raw, err := cursor.PeekSlice(1)
	if err != nil {
		return nil, err
	}
	Priority := (raw[0] >> 5) & 0x07
	Channel := (raw[0] >> 2) & 0x07
	Direction := (raw[0] >> 0) & 0x03
	value := &CodecSubbyte{
		Priority: Priority,
		Channel: Channel,
		Direction: Direction,
	}
	if err := cursor.Advance(1); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode writes the CodecSubbyte into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecSubbyte) Encode(w codec.SceSink) error {
	if err := w.WriteBytes([]byte{ byte((s.Priority & 0x07) << 5 | (s.Channel & 0x07) << 2 | (s.Direction & 0x03) << 0) }); err != nil {
		return err
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecSubbyte) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 1)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
