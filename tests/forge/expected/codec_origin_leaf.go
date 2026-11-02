// SCE-MAP: codec_origin_leaf:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_origin_leaf

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecOriginLeaf represents the codec frame layout.
type CodecOriginLeaf struct {
	Len uint8
	Data []byte
}

// DecodeCodecOriginLeaf decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecOriginLeaf(cursor *codec.SceCursor) (*CodecOriginLeaf, error) {
	frameLen := cursor.Remaining()
	if frameLen < 1 {
		return nil, codec.ErrNeedMoreBytes
	}
	raw, err := cursor.PeekSlice(frameLen)
	if err != nil {
		return nil, err
	}
	Len := raw[0]
	if Len > 16 {
		return nil, codec.ErrLengthOutOfRange
	}
	if frameLen < 1+int(Len) {
		return nil, codec.ErrNeedMoreBytes
	}
	Data := raw[1:1+int(Len)]
	value := &CodecOriginLeaf{
		Len: Len,
		Data: Data,
	}
	if err := cursor.Advance(frameLen); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode writes the CodecOriginLeaf into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecOriginLeaf) Encode(w codec.SceSink) error {
	if len(s.Data) > 16 {
		return codec.ErrLengthOutOfRange
	}
	if err := w.WriteBytes([]byte{ byte(s.Len) }); err != nil {
		return err
	}
	if err := w.WriteBytes(s.Data); err != nil {
		return err
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice, or the
// error Encode refuses the value with before its first byte: a field's
// presence disagrees with the `sce:present-if` that gates it
// (codec.ErrPresentIfMismatch), or a byte string, text, list or chain lies
// outside its bound (codec.ErrLengthOutOfRange, codec.ErrTooManyElements).
// Callers targeting zero-alloc hot paths should call Encode directly against a caller-owned sink (e.g.
// BoundedSink over a stack buffer).
func (s *CodecOriginLeaf) EncodeToBytes() ([]byte, error) {
	_dst := make([]byte, 0, 17)
	if err := s.Encode(codec.NewBytesSink(&_dst)); err != nil {
		return nil, err
	}
	return _dst, nil
}
