// SCE-MAP: codec_simple_frame:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_simple_frame

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecSimpleFrameSource is what the encode of CodecSimpleFrame reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecSimpleFrame is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecSimpleFrameSource interface {
	MsgId() uint8
	Length() uint8
	Payload() uint16
}

// CodecSimpleFrame represents the codec frame layout.
type CodecSimpleFrame struct {
	msgId uint8
	length uint8
	payload uint16
}

// MakeCodecSimpleFrame returns the CodecSimpleFrame that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecSimpleFrame, or as its zero value, and is read through its methods.
func MakeCodecSimpleFrame(msgId uint8, length uint8, payload uint16) CodecSimpleFrame {
	return CodecSimpleFrame{msgId: msgId, length: length, payload: payload}
}

// MsgId returns the element the value presents as MsgId.
func (s *CodecSimpleFrame) MsgId() uint8 {
	return s.msgId
}

// Length returns the element the value presents as Length.
func (s *CodecSimpleFrame) Length() uint8 {
	return s.length
}

// Payload returns the element the value presents as Payload.
func (s *CodecSimpleFrame) Payload() uint16 {
	return s.payload
}

// EncodeSource writes any source of CodecSimpleFrame into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecSimpleFrameSource, w codec.SceSink) error {
	if err := w.WriteBytes([]byte{byte(src.MsgId())}); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{byte(src.Length())}); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{byte(src.Payload() >> 8 & 0xFF), byte(src.Payload() & 0xFF)}); err != nil {
		return err
	}
	return nil
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
		msgId: MsgId,
		length: Length,
		payload: Payload,
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
	return EncodeSource(s, w)
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
