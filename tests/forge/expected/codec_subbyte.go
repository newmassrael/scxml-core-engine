// SCE-MAP: codec_subbyte:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_subbyte

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecSubbyteSource is what the encode of CodecSubbyte reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecSubbyte is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecSubbyteSource interface {
	Priority() uint8
	Channel() uint8
	Direction() uint8
}

// CodecSubbyte represents the codec frame layout.
type CodecSubbyte struct {
	priority uint8
	channel uint8
	direction uint8
}

// MakeCodecSubbyte returns the CodecSubbyte that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecSubbyte, or as its zero value, and is read through its methods.
func MakeCodecSubbyte(priority uint8, channel uint8, direction uint8) CodecSubbyte {
	return CodecSubbyte{priority: priority, channel: channel, direction: direction}
}

// Priority returns the element the value presents as Priority.
func (s *CodecSubbyte) Priority() uint8 {
	return s.priority
}

// Channel returns the element the value presents as Channel.
func (s *CodecSubbyte) Channel() uint8 {
	return s.channel
}

// Direction returns the element the value presents as Direction.
func (s *CodecSubbyte) Direction() uint8 {
	return s.direction
}

// EncodeSource writes any source of CodecSubbyte into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecSubbyteSource, w codec.SceSink) error {
	if err := w.WriteBytes([]byte{byte((src.Priority() & 0x07) << 5 | (src.Channel() & 0x07) << 2 | (src.Direction() & 0x03) << 0)}); err != nil {
		return err
	}
	return nil
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
		priority: Priority,
		channel: Channel,
		direction: Direction,
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
	return EncodeSource(s, w)
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
