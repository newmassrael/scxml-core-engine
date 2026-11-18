// SCE-MAP: codec_chain_has_marker_slice:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_chain_has_marker_slice

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecChainHasMarkerSliceDataBound is the bound of the data element of CodecChainHasMarkerSlice: 0 to 256 bytes
// (docs/adr/0077 §3). A Bytes of it is made only within it.
type CodecChainHasMarkerSliceDataBound struct{}

// Least is the fewest bytes the element holds.
func (CodecChainHasMarkerSliceDataBound) Least() int { return 0 }

// Most is the most bytes the element holds.
func (CodecChainHasMarkerSliceDataBound) Most() int { return 256 }

// CodecChainHasMarkerSliceSource is what the encode of CodecChainHasMarkerSlice reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecChainHasMarkerSlice is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecChainHasMarkerSliceSource interface {
	Kind() uint16
	Data() codec.Bytes[CodecChainHasMarkerSliceDataBound]
}

// CodecChainHasMarkerSlice represents the codec frame layout.
type CodecChainHasMarkerSlice struct {
	kind uint16
	data codec.Bytes[CodecChainHasMarkerSliceDataBound]
}

// MakeCodecChainHasMarkerSlice returns the CodecChainHasMarkerSlice that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecChainHasMarkerSlice, or as its zero value, and is read through its methods.
func MakeCodecChainHasMarkerSlice(kind uint16, data codec.Bytes[CodecChainHasMarkerSliceDataBound]) CodecChainHasMarkerSlice {
	return CodecChainHasMarkerSlice{kind: kind, data: data}
}

// Kind returns the element the value presents as Kind.
func (s *CodecChainHasMarkerSlice) Kind() uint16 {
	return s.kind
}

// Data returns the element the value presents as Data.
func (s *CodecChainHasMarkerSlice) Data() codec.Bytes[CodecChainHasMarkerSliceDataBound] {
	return s.data
}

// WireSize returns the size the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecChainHasMarkerSlice) WireSize() uint64 {
	return uint64(s.Data().Len())
}

// EncodeSource writes any source of CodecChainHasMarkerSlice into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives size from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecChainHasMarkerSliceSource, w codec.SceSink) error {
	_dSize := uint64(src.Data().Len())
	if err := codec.WriteVLEU16(w, uint16(src.Kind())); err != nil {
		return err
	}
	if err := codec.WriteVLEU64(w, uint64(_dSize)); err != nil {
		return err
	}
	if err := w.WriteBytes(src.Data().Bytes()); err != nil {
		return err
	}
	return nil
}

// DecodeCodecChainHasMarkerSlice decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecChainHasMarkerSlice(cursor *codec.SceCursor) (*CodecChainHasMarkerSlice, error) {
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
	Kind, err := cursor.ReadVLEU16()
	if err != nil { return nil, err }
	Size, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var Data []byte
	{
		_len := Size
		if _len > 256 {
			return nil, codec.ErrLengthOutOfRange
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
	_bData, err := codec.MakeBytes[CodecChainHasMarkerSliceDataBound](Data)
	if err != nil {
		return nil, err
	}
	return &CodecChainHasMarkerSlice{
		kind: Kind,
		data: _bData,
	}, nil
}

// Encode writes the CodecChainHasMarkerSlice into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecChainHasMarkerSlice) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecChainHasMarkerSlice) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 268)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
