// SCE-MAP: codec_repeat_elem:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_repeat_elem

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecRepeatElemSource is what the encode of CodecRepeatElem reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecRepeatElem is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecRepeatElemSource interface {
	Seq() uint16
}

// CodecRepeatElem represents the codec frame layout.
type CodecRepeatElem struct {
	seq uint16
}

// MakeCodecRepeatElem returns the CodecRepeatElem that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecRepeatElem, or as its zero value, and is read through its methods.
func MakeCodecRepeatElem(seq uint16) CodecRepeatElem {
	return CodecRepeatElem{seq: seq}
}

// Seq returns the element the value presents as Seq.
func (s *CodecRepeatElem) Seq() uint16 {
	return s.seq
}

// EncodeSource writes any source of CodecRepeatElem into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecRepeatElemSource, w codec.SceSink) error {
	if err := w.WriteBytes([]byte{byte(src.Seq() >> 8 & 0xFF), byte(src.Seq() & 0xFF)}); err != nil {
		return err
	}
	return nil
}

// DecodeCodecRepeatElem decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecRepeatElem(cursor *codec.SceCursor) (*CodecRepeatElem, error) {
	raw, err := cursor.PeekSlice(2)
	if err != nil {
		return nil, err
	}
	Seq := uint16(raw[0])<<8 | uint16(raw[1])
	value := &CodecRepeatElem{
		seq: Seq,
	}
	if err := cursor.Advance(2); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode writes the CodecRepeatElem into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecRepeatElem) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecRepeatElem) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 2)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
