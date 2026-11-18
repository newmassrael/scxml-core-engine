// SCE-MAP: codec_length_ref:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_length_ref

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecLengthRefPayloadBound is the bound of the payload element of CodecLengthRef: 0 to 32 bytes
// (docs/adr/0077 §3). A Bytes of it is made only within it.
type CodecLengthRefPayloadBound struct{}

// Least is the fewest bytes the element holds.
func (CodecLengthRefPayloadBound) Least() int { return 0 }

// Most is the most bytes the element holds.
func (CodecLengthRefPayloadBound) Most() int { return 32 }

// CodecLengthRefSource is what the encode of CodecLengthRef reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecLengthRef is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecLengthRefSource interface {
	MsgId() uint8
	Payload() codec.Bytes[CodecLengthRefPayloadBound]
}

// CodecLengthRef represents the codec frame layout.
type CodecLengthRef struct {
	msgId uint8
	payload codec.Bytes[CodecLengthRefPayloadBound]
}

// MakeCodecLengthRef returns the CodecLengthRef that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecLengthRef, or as its zero value, and is read through its methods.
func MakeCodecLengthRef(msgId uint8, payload codec.Bytes[CodecLengthRefPayloadBound]) CodecLengthRef {
	return CodecLengthRef{msgId: msgId, payload: payload}
}

// MsgId returns the element the value presents as MsgId.
func (s *CodecLengthRef) MsgId() uint8 {
	return s.msgId
}

// Payload returns the element the value presents as Payload.
func (s *CodecLengthRef) Payload() codec.Bytes[CodecLengthRefPayloadBound] {
	return s.payload
}

// WireLen returns the len the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecLengthRef) WireLen() uint8 {
	return uint8(s.Payload().Len())
}

// EncodeSource writes any source of CodecLengthRef into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives len from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecLengthRefSource, w codec.SceSink) error {
	_dLen := uint8(src.Payload().Len())
	if err := w.WriteBytes([]byte{byte(src.MsgId())}); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{byte(_dLen)}); err != nil {
		return err
	}
	if err := w.WriteBytes(src.Payload().Bytes()); err != nil {
		return err
	}
	return nil
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
	if Len > 32 {
		return nil, codec.ErrLengthOutOfRange
	}
	if frameLen < 2+int(Len) {
		return nil, codec.ErrNeedMoreBytes
	}
	Payload := raw[2:2+int(Len)]
	_bPayload, err := codec.MakeBytes[CodecLengthRefPayloadBound](Payload)
	if err != nil {
		return nil, err
	}
	value := &CodecLengthRef{
		msgId: MsgId,
		payload: _bPayload,
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
	return EncodeSource(s, w)
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
