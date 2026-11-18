// SCE-MAP: codec_length_ref_uint32_le:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_length_ref_uint32_le

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecLengthRefUint32LePayloadBound is the bound of the payload element of CodecLengthRefUint32Le: 0 to 1024 bytes
// (docs/adr/0077 §3). A Bytes of it is made only within it.
type CodecLengthRefUint32LePayloadBound struct{}

// Least is the fewest bytes the element holds.
func (CodecLengthRefUint32LePayloadBound) Least() int { return 0 }

// Most is the most bytes the element holds.
func (CodecLengthRefUint32LePayloadBound) Most() int { return 1024 }

// CodecLengthRefUint32LeSource is what the encode of CodecLengthRefUint32Le reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecLengthRefUint32Le is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecLengthRefUint32LeSource interface {
	Payload() codec.Bytes[CodecLengthRefUint32LePayloadBound]
}

// CodecLengthRefUint32Le represents the codec frame layout.
type CodecLengthRefUint32Le struct {
	payload codec.Bytes[CodecLengthRefUint32LePayloadBound]
}

// MakeCodecLengthRefUint32Le returns the CodecLengthRefUint32Le that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecLengthRefUint32Le, or as its zero value, and is read through its methods.
func MakeCodecLengthRefUint32Le(payload codec.Bytes[CodecLengthRefUint32LePayloadBound]) CodecLengthRefUint32Le {
	return CodecLengthRefUint32Le{payload: payload}
}

// Payload returns the element the value presents as Payload.
func (s *CodecLengthRefUint32Le) Payload() codec.Bytes[CodecLengthRefUint32LePayloadBound] {
	return s.payload
}

// WirePayloadLen returns the payload_len the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecLengthRefUint32Le) WirePayloadLen() uint32 {
	return uint32(s.Payload().Len())
}

// EncodeSource writes any source of CodecLengthRefUint32Le into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives payload_len from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecLengthRefUint32LeSource, w codec.SceSink) error {
	_dPayloadLen := uint32(src.Payload().Len())
	if err := w.WriteBytes([]byte{byte(_dPayloadLen & 0xFF), byte(_dPayloadLen >> 8 & 0xFF), byte(_dPayloadLen >> 16 & 0xFF), byte(_dPayloadLen >> 24 & 0xFF)}); err != nil {
		return err
	}
	if err := w.WriteBytes(src.Payload().Bytes()); err != nil {
		return err
	}
	return nil
}

// DecodeCodecLengthRefUint32Le decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecLengthRefUint32Le(cursor *codec.SceCursor) (*CodecLengthRefUint32Le, error) {
	frameLen := cursor.Remaining()
	if frameLen < 4 {
		return nil, codec.ErrNeedMoreBytes
	}
	raw, err := cursor.PeekSlice(frameLen)
	if err != nil {
		return nil, err
	}
	PayloadLen := uint32(raw[0]) | uint32(raw[1])<<8 | uint32(raw[2])<<16 | uint32(raw[3])<<24
	if PayloadLen > 1024 {
		return nil, codec.ErrLengthOutOfRange
	}
	if frameLen < 4+int(PayloadLen) {
		return nil, codec.ErrNeedMoreBytes
	}
	Payload := raw[4:4+int(PayloadLen)]
	_bPayload, err := codec.MakeBytes[CodecLengthRefUint32LePayloadBound](Payload)
	if err != nil {
		return nil, err
	}
	value := &CodecLengthRefUint32Le{
		payload: _bPayload,
	}
	// The bytes it read and no more (RFC §synth-5-B B3, as the Rust decode
	// does): the end of the last length-ref field, at least the fixed fields'
	// bytes, so that a codec holding this one as a body, an entry or an arm
	// decodes on after it. Each end is an argument of `max`, where gofmt
	// spells it as the guard above does.
	consumed := 4
	consumed = max(consumed, 4+int(PayloadLen))
	if err := cursor.Advance(consumed); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode writes the CodecLengthRefUint32Le into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecLengthRefUint32Le) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecLengthRefUint32Le) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 1028)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
