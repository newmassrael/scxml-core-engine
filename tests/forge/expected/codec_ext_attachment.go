// SCE-MAP: codec_ext_attachment:27

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_ext_attachment

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecExtAttachment represents the codec frame layout.
type CodecExtAttachment struct {
	Length uint8
	Body []byte
}

// DecodeCodecExtAttachment decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecExtAttachment(cursor *codec.SceCursor) (*CodecExtAttachment, error) {
	frameLen := cursor.Remaining()
	if frameLen < 1 {
		return nil, codec.ErrNeedMoreBytes
	}
	raw, err := cursor.PeekSlice(frameLen)
	if err != nil {
		return nil, err
	}
	Length := raw[0]
	Body := raw[1:1+int(Length)]
	value := &CodecExtAttachment{
		Length: Length,
		Body: Body,
	}
	if err := cursor.Advance(frameLen); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode writes the CodecExtAttachment into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecExtAttachment) Encode(w codec.SceSink) error {
	if err := w.WriteBytes([]byte{ byte(s.Length) }); err != nil {
		return err
	}
	if err := w.WriteBytes(s.Body); err != nil {
		return err
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecExtAttachment) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 65)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
