// SCE-MAP: codec_zenoh_close:16

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_close

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohClose represents the codec frame layout.
type CodecZenohClose struct {
	Reason uint8
}

// DecodeCodecZenohClose decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohClose(cursor *codec.SceCursor) (*CodecZenohClose, error) {
	raw, err := cursor.PeekSlice(1)
	if err != nil {
		return nil, err
	}
	Reason := raw[0]
	value := &CodecZenohClose{
		Reason: Reason,
	}
	if err := cursor.Advance(1); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode writes the CodecZenohClose into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohClose) Encode(w codec.SceSink) error {
	if err := w.WriteBytes([]byte{ byte(s.Reason) }); err != nil {
		return err
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohClose) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 1)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
