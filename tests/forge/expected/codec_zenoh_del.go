// SCE-MAP: codec_zenoh_del:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_del

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohDelSource is what the encode of CodecZenohDel reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohDel is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohDelSource interface {
}

// CodecZenohDel represents the codec frame layout.
type CodecZenohDel struct {
}

// MakeCodecZenohDel returns the CodecZenohDel that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohDel, or as its zero value, and is read through its methods.
func MakeCodecZenohDel() CodecZenohDel {
	return CodecZenohDel{}
}

// EncodeSource writes any source of CodecZenohDel into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohDelSource, w codec.SceSink) error {
	return nil
}

// DecodeCodecZenohDel decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohDel(cursor *codec.SceCursor) (*CodecZenohDel, error) {
	// RFC §synth-5-B empty body — zero-byte payload, no cursor work.
	_ = cursor
	return &CodecZenohDel{}, nil
}

// Encode writes the CodecZenohDel into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohDel) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohDel) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 0)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
