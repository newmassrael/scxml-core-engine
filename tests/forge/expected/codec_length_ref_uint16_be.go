// SCE-MAP: codec_length_ref_uint16_be:12

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_length_ref_uint16_be

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecLengthRefUint16Be represents the codec frame layout.
type CodecLengthRefUint16Be struct {
	PayloadLen uint16
	Payload []byte
}

// DecodeCodecLengthRefUint16Be decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecLengthRefUint16Be(cursor *codec.SceCursor) (*CodecLengthRefUint16Be, error) {
	frameLen := cursor.Remaining()
	if frameLen < 2 {
		return nil, codec.ErrNeedMoreBytes
	}
	raw, err := cursor.PeekSlice(frameLen)
	if err != nil {
		return nil, err
	}
	PayloadLen := uint16(raw[0])<<8 | uint16(raw[1])
	Payload := raw[2:2+int(PayloadLen)]
	value := &CodecLengthRefUint16Be{
		PayloadLen: PayloadLen,
		Payload: Payload,
	}
	if err := cursor.Advance(frameLen); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode writes the CodecLengthRefUint16Be into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecLengthRefUint16Be) Encode(w codec.SceSink) error {
	if err := w.WriteBytes([]byte{ byte(s.PayloadLen >> 8 & 0xFF) }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ byte(s.PayloadLen & 0xFF) }); err != nil {
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
func (s *CodecLengthRefUint16Be) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 1026)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
