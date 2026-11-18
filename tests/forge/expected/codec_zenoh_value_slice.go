// SCE-MAP: codec_zenoh_value_slice:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_value_slice

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohValueSliceDataBound is the bound of the data element of CodecZenohValueSlice: 0 to 4294967295 bytes
// (docs/adr/0077 §3). A Bytes of it is made only within it.
type CodecZenohValueSliceDataBound struct{}

// Least is the fewest bytes the element holds.
func (CodecZenohValueSliceDataBound) Least() int { return 0 }

// Most is the most bytes the element holds.
func (CodecZenohValueSliceDataBound) Most() int { return 4294967295 }

// CodecZenohValueSliceSource is what the encode of CodecZenohValueSlice reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohValueSlice is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohValueSliceSource interface {
	Kind() uint8
	Data() codec.Bytes[CodecZenohValueSliceDataBound]
}

// CodecZenohValueSlice represents the codec frame layout.
type CodecZenohValueSlice struct {
	kind uint8
	data codec.Bytes[CodecZenohValueSliceDataBound]
}

// MakeCodecZenohValueSlice returns the CodecZenohValueSlice that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohValueSlice, or as its zero value, and is read through its methods.
func MakeCodecZenohValueSlice(kind uint8, data codec.Bytes[CodecZenohValueSliceDataBound]) CodecZenohValueSlice {
	return CodecZenohValueSlice{kind: kind, data: data}
}

// Kind returns the element the value presents as Kind.
func (s *CodecZenohValueSlice) Kind() uint8 {
	return s.kind
}

// Data returns the element the value presents as Data.
func (s *CodecZenohValueSlice) Data() codec.Bytes[CodecZenohValueSliceDataBound] {
	return s.data
}

// WireLen returns the len the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecZenohValueSlice) WireLen() uint64 {
	return uint64(s.Data().Len())
}

// EncodeSource writes any source of CodecZenohValueSlice into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives len from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohValueSliceSource, w codec.SceSink) error {
	_dLen := uint64(src.Data().Len())
	if err := w.WriteBytes([]byte{byte(src.Kind())}); err != nil {
		return err
	}
	if err := codec.WriteVLEU64(w, uint64(_dLen)); err != nil {
		return err
	}
	if err := w.WriteBytes(src.Data().Bytes()); err != nil {
		return err
	}
	return nil
}

// DecodeCodecZenohValueSlice decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohValueSlice(cursor *codec.SceCursor) (*CodecZenohValueSlice, error) {
	// Streaming cursor decode (SSOT selection: `needs_streaming`).
	// The positional `raw[byte_off]` path is valid only when every
	// field's absolute offset is fixed at codegen time; this branch
	// handles every codec where it is not — present-if-gated fields
	// (runtime presence, held by the runtime's optional type), VLE / repeat / TLV-chain /
	// embed fields (runtime width), string fields (UTF-8 decode), and a
	// fixed field after a variable-length payload (offset depends on the
	// payload length). Each field reads its own bytes from the cursor and
	// advances past what it consumed. Per-field `is_repeat` /
	// `is_tlv_chain` / `is_embed` route to their dedicated helpers; every
	// other field flows through `present_if_decode_stmt`.
	var Kind uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Kind = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	Len, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var Data []byte
	{
		_len := Len
		if _len > 4294967295 {
			return nil, codec.ErrLengthOutOfRange
		}
		if uint64(_len) > uint64(^uint(0)>>1)-0 {
			return nil, codec.ErrNeedMoreBytes
		}
		_n := int(_len)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Data = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	_bData, err := codec.MakeBytes[CodecZenohValueSliceDataBound](Data)
	if err != nil {
		return nil, err
	}
	return &CodecZenohValueSlice{
		kind: Kind,
		data: _bData,
	}, nil
}

// Encode writes the CodecZenohValueSlice into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohValueSlice) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohValueSlice) EncodeToBytes() []byte {
	_dst := make([]byte, 0)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
