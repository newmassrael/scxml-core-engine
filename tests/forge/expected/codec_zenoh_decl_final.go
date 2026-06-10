// SCE-MAP: codec_zenoh_decl_final:19

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_decl_final

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohDeclFinal represents the codec frame layout.
type CodecZenohDeclFinal struct {
}

// DecodeCodecZenohDeclFinal decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohDeclFinal(cursor *codec.SceCursor) (*CodecZenohDeclFinal, error) {
	// RFC §synth-5-B empty body — zero-byte payload, no cursor work.
	_ = cursor
	return &CodecZenohDeclFinal{}, nil
}

// Encode writes the CodecZenohDeclFinal into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohDeclFinal) Encode(w codec.SceSink) error {
	// RFC §synth-5-B empty body — zero-byte payload.
	_ = w
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohDeclFinal) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 0)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
