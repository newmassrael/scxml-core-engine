// SCE-MAP: codec_length_ref:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_length_ref

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecLengthRef represents the codec frame layout.
type CodecLengthRef struct {
	MsgId uint8
	Len uint8
	Payload []byte
}

// DecodeCodecLengthRef decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecLengthRef(cursor *codec.SceCursor) (*CodecLengthRef, error) {
	frameLen := cursor.Remaining()
	if frameLen < 2 {
		return nil, codec.ErrNeedMoreBytes
	}
	raw, err := cursor.PeekSlice(frameLen)
	if err != nil {
		return nil, err
	}
	MsgId := raw[0]
	Len := raw[1]
	Payload := raw[2:2+int(Len)]
	value := &CodecLengthRef{
		MsgId: MsgId,
		Len: Len,
		Payload: Payload,
	}
	if err := cursor.Advance(frameLen); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode writes the CodecLengthRef into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecLengthRef) Encode(w codec.SceSink) error {
	if err := w.WriteBytes([]byte{ byte(s.MsgId) }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ byte(s.Len) }); err != nil {
		return err
	}
	if err := w.WriteBytes(s.Payload); err != nil {
		return err
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecLengthRef) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 34)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
