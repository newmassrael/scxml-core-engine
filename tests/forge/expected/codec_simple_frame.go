// SCE-MAP: codec_simple_frame:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_simple_frame

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecSimpleFrame represents the codec frame layout.
type CodecSimpleFrame struct {
	MsgId uint8
	Length uint8
	Payload uint16
}

// DecodeCodecSimpleFrame decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecSimpleFrame(cursor *codec.SceCursor) (*CodecSimpleFrame, error) {
	raw, err := cursor.PeekSlice(4)
	if err != nil {
		return nil, err
	}
	MsgId := raw[0]
	Length := raw[1]
	Payload := uint16(raw[2])<<8 | uint16(raw[3])
	value := &CodecSimpleFrame{
		MsgId: MsgId,
		Length: Length,
		Payload: Payload,
	}
	if err := cursor.Advance(4); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode writes the CodecSimpleFrame into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecSimpleFrame) Encode(w codec.SceSink) error {
	if err := w.WriteBytes([]byte{ byte(s.MsgId) }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ byte(s.Length) }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ byte(s.Payload >> 8 & 0xFF) }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ byte(s.Payload & 0xFF) }); err != nil {
		return err
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecSimpleFrame) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 4)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
