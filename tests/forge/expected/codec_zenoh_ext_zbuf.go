// SCE-MAP: codec_zenoh_ext_zbuf:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_ext_zbuf

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohExtZbufValueBound is the bound of the value element of CodecZenohExtZbuf: 0 to 4294967295 bytes
// (docs/adr/0077 §3). A Bytes of it is made only within it.
type CodecZenohExtZbufValueBound struct{}

// Least is the fewest bytes the element holds.
func (CodecZenohExtZbufValueBound) Least() int { return 0 }

// Most is the most bytes the element holds.
func (CodecZenohExtZbufValueBound) Most() int { return 4294967295 }

// CodecZenohExtZbufSource is what the encode of CodecZenohExtZbuf reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohExtZbuf is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohExtZbufSource interface {
	Value() codec.Bytes[CodecZenohExtZbufValueBound]
}

// CodecZenohExtZbuf represents the codec frame layout.
type CodecZenohExtZbuf struct {
	value codec.Bytes[CodecZenohExtZbufValueBound]
}

// MakeCodecZenohExtZbuf returns the CodecZenohExtZbuf that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohExtZbuf, or as its zero value, and is read through its methods.
func MakeCodecZenohExtZbuf(value codec.Bytes[CodecZenohExtZbufValueBound]) CodecZenohExtZbuf {
	return CodecZenohExtZbuf{value: value}
}

// Value returns the element the value presents as Value.
func (s *CodecZenohExtZbuf) Value() codec.Bytes[CodecZenohExtZbufValueBound] {
	return s.value
}

// WireValueLen returns the value_len the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecZenohExtZbuf) WireValueLen() uint64 {
	return uint64(s.Value().Len())
}

// EncodeSource writes any source of CodecZenohExtZbuf into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives value_len from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohExtZbufSource, w codec.SceSink) error {
	_dValueLen := uint64(src.Value().Len())
	if err := codec.WriteVLEU64(w, uint64(_dValueLen)); err != nil {
		return err
	}
	if err := w.WriteBytes(src.Value().Bytes()); err != nil {
		return err
	}
	return nil
}

// DecodeCodecZenohExtZbuf decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohExtZbuf(cursor *codec.SceCursor) (*CodecZenohExtZbuf, error) {
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
	ValueLen, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var Value []byte
	{
		_len := ValueLen
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
		Value = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	_bValue, err := codec.MakeBytes[CodecZenohExtZbufValueBound](Value)
	if err != nil {
		return nil, err
	}
	return &CodecZenohExtZbuf{
		value: _bValue,
	}, nil
}

// Encode writes the CodecZenohExtZbuf into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohExtZbuf) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohExtZbuf) EncodeToBytes() []byte {
	_dst := make([]byte, 0)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
