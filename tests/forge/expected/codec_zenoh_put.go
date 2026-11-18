// SCE-MAP: codec_zenoh_put:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_put

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohPutSource is what the encode of CodecZenohPut reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohPut is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohPutSource interface {
	Payload() uint8
}

// CodecZenohPut represents the codec frame layout.
type CodecZenohPut struct {
	payload uint8
}

// MakeCodecZenohPut returns the CodecZenohPut that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohPut, or as its zero value, and is read through its methods.
func MakeCodecZenohPut(payload uint8) CodecZenohPut {
	return CodecZenohPut{payload: payload}
}

// Payload returns the element the value presents as Payload.
func (s *CodecZenohPut) Payload() uint8 {
	return s.payload
}

// EncodeSource writes any source of CodecZenohPut into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohPutSource, w codec.SceSink) error {
	if err := w.WriteBytes([]byte{byte(src.Payload())}); err != nil {
		return err
	}
	return nil
}

// DecodeCodecZenohPut decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohPut(cursor *codec.SceCursor) (*CodecZenohPut, error) {
	raw, err := cursor.PeekSlice(1)
	if err != nil {
		return nil, err
	}
	Payload := raw[0]
	value := &CodecZenohPut{
		payload: Payload,
	}
	if err := cursor.Advance(1); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode writes the CodecZenohPut into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohPut) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohPut) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 1)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
