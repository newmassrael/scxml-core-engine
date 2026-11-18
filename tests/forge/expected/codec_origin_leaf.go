// SCE-MAP: codec_origin_leaf:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_origin_leaf

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecOriginLeafDataBound is the bound of the data element of CodecOriginLeaf: 0 to 16 bytes
// (docs/adr/0077 §3). A Bytes of it is made only within it.
type CodecOriginLeafDataBound struct{}

// Least is the fewest bytes the element holds.
func (CodecOriginLeafDataBound) Least() int { return 0 }

// Most is the most bytes the element holds.
func (CodecOriginLeafDataBound) Most() int { return 16 }

// CodecOriginLeafSource is what the encode of CodecOriginLeaf reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecOriginLeaf is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecOriginLeafSource interface {
	Data() codec.Bytes[CodecOriginLeafDataBound]
}

// CodecOriginLeaf represents the codec frame layout.
type CodecOriginLeaf struct {
	data codec.Bytes[CodecOriginLeafDataBound]
}

// MakeCodecOriginLeaf returns the CodecOriginLeaf that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecOriginLeaf, or as its zero value, and is read through its methods.
func MakeCodecOriginLeaf(data codec.Bytes[CodecOriginLeafDataBound]) CodecOriginLeaf {
	return CodecOriginLeaf{data: data}
}

// Data returns the element the value presents as Data.
func (s *CodecOriginLeaf) Data() codec.Bytes[CodecOriginLeafDataBound] {
	return s.data
}

// WireLen returns the len the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecOriginLeaf) WireLen() uint8 {
	return uint8(s.Data().Len())
}

// EncodeSource writes any source of CodecOriginLeaf into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives len from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecOriginLeafSource, w codec.SceSink) error {
	_dLen := uint8(src.Data().Len())
	if err := w.WriteBytes([]byte{byte(_dLen)}); err != nil {
		return err
	}
	if err := w.WriteBytes(src.Data().Bytes()); err != nil {
		return err
	}
	return nil
}

// DecodeCodecOriginLeaf decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecOriginLeaf(cursor *codec.SceCursor) (*CodecOriginLeaf, error) {
	frameLen := cursor.Remaining()
	if frameLen < 1 {
		return nil, codec.ErrNeedMoreBytes
	}
	raw, err := cursor.PeekSlice(frameLen)
	if err != nil {
		return nil, err
	}
	Len := raw[0]
	if Len > 16 {
		return nil, codec.ErrLengthOutOfRange
	}
	if frameLen < 1+int(Len) {
		return nil, codec.ErrNeedMoreBytes
	}
	Data := raw[1:1+int(Len)]
	_bData, err := codec.MakeBytes[CodecOriginLeafDataBound](Data)
	if err != nil {
		return nil, err
	}
	value := &CodecOriginLeaf{
		data: _bData,
	}
	if err := cursor.Advance(frameLen); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode writes the CodecOriginLeaf into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecOriginLeaf) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecOriginLeaf) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 17)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
