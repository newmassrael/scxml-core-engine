// SCE-MAP: codec_scout_zid_body:35 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_scout_zid_body

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecScoutZidBodyZidBound is the bound of the zid element of CodecScoutZidBody: 1 to 16 bytes
// (docs/adr/0077 §3). A Bytes of it is made only within it.
type CodecScoutZidBodyZidBound struct{}

// Least is the fewest bytes the element holds.
func (CodecScoutZidBodyZidBound) Least() int { return 1 }

// Most is the most bytes the element holds.
func (CodecScoutZidBodyZidBound) Most() int { return 16 }

// CodecScoutZidBodySource is what the encode of CodecScoutZidBody reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecScoutZidBody is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecScoutZidBodySource interface {
	Zid() codec.Bytes[CodecScoutZidBodyZidBound]
}

// CodecScoutZidBody represents the codec frame layout.
type CodecScoutZidBody struct {
	zid codec.Bytes[CodecScoutZidBodyZidBound]
}

// MakeCodecScoutZidBody returns the CodecScoutZidBody that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecScoutZidBody, or as its zero value, and is read through its methods.
func MakeCodecScoutZidBody(zid codec.Bytes[CodecScoutZidBodyZidBound]) CodecScoutZidBody {
	return CodecScoutZidBody{zid: zid}
}

// Zid returns the element the value presents as Zid.
func (s *CodecScoutZidBody) Zid() codec.Bytes[CodecScoutZidBodyZidBound] {
	return s.zid
}

// WireZidLenM1 returns the zid_len_m1 the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecScoutZidBody) WireZidLenM1() uint8 {
	return uint8(s.Zid().Len() - 1)
}

// EncodeSource writes any source of CodecScoutZidBody into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives zid_len_m1 from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecScoutZidBodySource, w codec.SceSink) error {
	_dZidLenM1 := uint8(src.Zid().Len() - 1)
	if err := w.WriteBytes([]byte{byte(_dZidLenM1)}); err != nil {
		return err
	}
	if err := w.WriteBytes(src.Zid().Bytes()); err != nil {
		return err
	}
	return nil
}

// DecodeCodecScoutZidBody decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecScoutZidBody(cursor *codec.SceCursor) (*CodecScoutZidBody, error) {
	frameLen := cursor.Remaining()
	if frameLen < 1 {
		return nil, codec.ErrNeedMoreBytes
	}
	raw, err := cursor.PeekSlice(frameLen)
	if err != nil {
		return nil, err
	}
	ZidLenM1 := raw[0]
	if ZidLenM1 > 15 {
		return nil, codec.ErrLengthOutOfRange
	}
	if frameLen < 1+int(ZidLenM1)+1 {
		return nil, codec.ErrNeedMoreBytes
	}
	Zid := raw[1:1+int(ZidLenM1)+1]
	_bZid, err := codec.MakeBytes[CodecScoutZidBodyZidBound](Zid)
	if err != nil {
		return nil, err
	}
	value := &CodecScoutZidBody{
		zid: _bZid,
	}
	// The bytes it read and no more (RFC §synth-5-B B3, as the Rust decode
	// does): the end of the last length-ref field, at least the fixed fields'
	// bytes, so that a codec holding this one as a body, an entry or an arm
	// decodes on after it. Each end is an argument of `max`, where gofmt
	// spells it as the guard above does.
	consumed := 1
	consumed = max(consumed, 1+int(ZidLenM1)+1)
	if err := cursor.Advance(consumed); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode writes the CodecScoutZidBody into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecScoutZidBody) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecScoutZidBody) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 17)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
