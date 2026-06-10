// SCE-MAP: codec_variant_session_open:5

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_variant_session_open

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecVariantSessionOpen represents the codec frame layout.
type CodecVariantSessionOpen struct {
	Version uint16
}

// DecodeCodecVariantSessionOpen decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecVariantSessionOpen(cursor *codec.SceCursor) (*CodecVariantSessionOpen, error) {
	raw, err := cursor.PeekSlice(2)
	if err != nil {
		return nil, err
	}
	Version := uint16(raw[0])<<8 | uint16(raw[1])
	value := &CodecVariantSessionOpen{
		Version: Version,
	}
	if err := cursor.Advance(2); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode writes the CodecVariantSessionOpen into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecVariantSessionOpen) Encode(w codec.SceSink) error {
	if err := w.WriteBytes([]byte{ byte(s.Version >> 8 & 0xFF) }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ byte(s.Version & 0xFF) }); err != nil {
		return err
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecVariantSessionOpen) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 2)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
