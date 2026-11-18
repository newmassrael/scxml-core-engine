// SCE-MAP: codec_tail:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_tail

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecTailPayloadBound is the bound of the payload element of CodecTail: 0 to 32 bytes
// (docs/adr/0077 §3). A Bytes of it is made only within it.
type CodecTailPayloadBound struct{}

// Least is the fewest bytes the element holds.
func (CodecTailPayloadBound) Least() int { return 0 }

// Most is the most bytes the element holds.
func (CodecTailPayloadBound) Most() int { return 32 }

// CodecTailSource is what the encode of CodecTail reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecTail is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecTailSource interface {
	MsgId() uint8
	Status() uint8
	Payload() codec.Bytes[CodecTailPayloadBound]
}

// CodecTail represents the codec frame layout.
type CodecTail struct {
	msgId uint8
	status uint8
	payload codec.Bytes[CodecTailPayloadBound]
}

// MakeCodecTail returns the CodecTail that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecTail, or as its zero value, and is read through its methods.
func MakeCodecTail(msgId uint8, status uint8, payload codec.Bytes[CodecTailPayloadBound]) CodecTail {
	return CodecTail{msgId: msgId, status: status, payload: payload}
}

// MsgId returns the element the value presents as MsgId.
func (s *CodecTail) MsgId() uint8 {
	return s.msgId
}

// Status returns the element the value presents as Status.
func (s *CodecTail) Status() uint8 {
	return s.status
}

// Payload returns the element the value presents as Payload.
func (s *CodecTail) Payload() codec.Bytes[CodecTailPayloadBound] {
	return s.payload
}

// EncodeSource writes any source of CodecTail into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecTailSource, w codec.SceSink) error {
	if err := w.WriteBytes([]byte{byte(src.MsgId())}); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{byte(src.Status())}); err != nil {
		return err
	}
	if err := w.WriteBytes(src.Payload().Bytes()); err != nil {
		return err
	}
	return nil
}

// DecodeCodecTail decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecTail(cursor *codec.SceCursor) (*CodecTail, error) {
	frameLen := cursor.Remaining()
	if frameLen < 2 {
		return nil, codec.ErrNeedMoreBytes
	}
	raw, err := cursor.PeekSlice(frameLen)
	if err != nil {
		return nil, err
	}
	MsgId := raw[0]
	Status := raw[1]
	if (frameLen - 2) > 32 {
		return nil, codec.ErrLengthOutOfRange
	}
	Payload := raw[2:]
	_bPayload, err := codec.MakeBytes[CodecTailPayloadBound](Payload)
	if err != nil {
		return nil, err
	}
	value := &CodecTail{
		msgId: MsgId,
		status: Status,
		payload: _bPayload,
	}
	if err := cursor.Advance(frameLen); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode writes the CodecTail into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecTail) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecTail) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 34)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
