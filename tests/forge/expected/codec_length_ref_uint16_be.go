// SCE-MAP: codec_length_ref_uint16_be:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_length_ref_uint16_be

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecLengthRefUint16BePayloadBound is the bound of the payload element of CodecLengthRefUint16Be: 0 to 1024 bytes
// (docs/adr/0077 §3). A Bytes of it is made only within it.
type CodecLengthRefUint16BePayloadBound struct{}

// Least is the fewest bytes the element holds.
func (CodecLengthRefUint16BePayloadBound) Least() int { return 0 }

// Most is the most bytes the element holds.
func (CodecLengthRefUint16BePayloadBound) Most() int { return 1024 }

// CodecLengthRefUint16BeSource is what the encode of CodecLengthRefUint16Be reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecLengthRefUint16Be is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecLengthRefUint16BeSource interface {
	Payload() codec.Bytes[CodecLengthRefUint16BePayloadBound]
}

// CodecLengthRefUint16Be represents the codec frame layout.
type CodecLengthRefUint16Be struct {
	payload codec.Bytes[CodecLengthRefUint16BePayloadBound]
}

// MakeCodecLengthRefUint16Be returns the CodecLengthRefUint16Be that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecLengthRefUint16Be, or as its zero value, and is read through its methods.
func MakeCodecLengthRefUint16Be(payload codec.Bytes[CodecLengthRefUint16BePayloadBound]) CodecLengthRefUint16Be {
	return CodecLengthRefUint16Be{payload: payload}
}

// Payload returns the element the value presents as Payload.
func (s *CodecLengthRefUint16Be) Payload() codec.Bytes[CodecLengthRefUint16BePayloadBound] {
	return s.payload
}

// WirePayloadLen returns the payload_len the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecLengthRefUint16Be) WirePayloadLen() uint16 {
	return uint16(s.Payload().Len())
}

// EncodeSource writes any source of CodecLengthRefUint16Be into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives payload_len from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecLengthRefUint16BeSource, w codec.SceSink) error {
	_dPayloadLen := uint16(src.Payload().Len())
	if err := w.WriteBytes([]byte{byte(_dPayloadLen >> 8 & 0xFF), byte(_dPayloadLen & 0xFF)}); err != nil {
		return err
	}
	if err := w.WriteBytes(src.Payload().Bytes()); err != nil {
		return err
	}
	return nil
}

// DecodeCodecLengthRefUint16Be decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
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
	if PayloadLen > 1024 {
		return nil, codec.ErrLengthOutOfRange
	}
	if frameLen < 2+int(PayloadLen) {
		return nil, codec.ErrNeedMoreBytes
	}
	Payload := raw[2:2+int(PayloadLen)]
	_bPayload, err := codec.MakeBytes[CodecLengthRefUint16BePayloadBound](Payload)
	if err != nil {
		return nil, err
	}
	value := &CodecLengthRefUint16Be{
		payload: _bPayload,
	}
	// The bytes it read and no more (RFC §synth-5-B B3, as the Rust decode
	// does): the end of the last length-ref field, at least the fixed fields'
	// bytes, so that a codec holding this one as a body, an entry or an arm
	// decodes on after it. Each end is an argument of `max`, where gofmt
	// spells it as the guard above does.
	consumed := 2
	consumed = max(consumed, 2+int(PayloadLen))
	if err := cursor.Advance(consumed); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode writes the CodecLengthRefUint16Be into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecLengthRefUint16Be) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
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
