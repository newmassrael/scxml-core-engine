// SCE-MAP: codec_zenoh_keep_alive:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_keep_alive

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohKeepAliveSource is what the encode of CodecZenohKeepAlive reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohKeepAlive is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohKeepAliveSource interface {
}

// CodecZenohKeepAlive represents the codec frame layout.
type CodecZenohKeepAlive struct {
}

// MakeCodecZenohKeepAlive returns the CodecZenohKeepAlive that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohKeepAlive, or as its zero value, and is read through its methods.
func MakeCodecZenohKeepAlive() CodecZenohKeepAlive {
	return CodecZenohKeepAlive{}
}

// EncodeSource writes any source of CodecZenohKeepAlive into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohKeepAliveSource, w codec.SceSink) error {
	return nil
}

// DecodeCodecZenohKeepAlive decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohKeepAlive(cursor *codec.SceCursor) (*CodecZenohKeepAlive, error) {
	// RFC §synth-5-B empty body — zero-byte payload, no cursor work.
	_ = cursor
	return &CodecZenohKeepAlive{}, nil
}

// Encode writes the CodecZenohKeepAlive into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohKeepAlive) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohKeepAlive) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 0)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
