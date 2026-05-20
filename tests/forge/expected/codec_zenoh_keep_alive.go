// SCE-MAP: codec_zenoh_keep_alive:10

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_keep_alive

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohKeepAlive represents the codec frame layout.
type CodecZenohKeepAlive struct {
}

// DecodeCodecZenohKeepAlive decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohKeepAlive(cursor *codec.SceCursor) (*CodecZenohKeepAlive, error) {
	// RFC §5.B B5-α empty body — zero-byte payload, no cursor work.
	_ = cursor
	return &CodecZenohKeepAlive{}, nil
}

// Encode writes the CodecZenohKeepAlive into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohKeepAlive) Encode(w codec.SceSink) error {
	// RFC §5.B B5-α empty body — zero-byte payload.
	_ = w
	return nil
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
