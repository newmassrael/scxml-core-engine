// SCE-MAP: codec_zenoh_encoding:69 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_encoding

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
	"unicode/utf8"
)

// CodecZenohEncodingSchemaBound is the bound of the schema element of CodecZenohEncoding: 0 to 255 UTF-8 bytes
// (docs/adr/0077 §3). A Text of it is made only within it.
type CodecZenohEncodingSchemaBound struct{}

// Least is the fewest UTF-8 bytes the element holds.
func (CodecZenohEncodingSchemaBound) Least() int { return 0 }

// Most is the most UTF-8 bytes the element holds.
func (CodecZenohEncodingSchemaBound) Most() int { return 255 }

// CodecZenohEncodingSource is what the encode of CodecZenohEncoding reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohEncoding is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohEncodingSource interface {
	PackedId() uint32
	Schema() optional.Of[codec.Text[CodecZenohEncodingSchemaBound]]
}

// CodecZenohEncoding represents the codec frame layout.
type CodecZenohEncoding struct {
	packedId uint32
	schema optional.Of[codec.Text[CodecZenohEncodingSchemaBound]]
}

// MakeCodecZenohEncoding returns the CodecZenohEncoding that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohEncoding, or as its zero value, and is read through its methods.
func MakeCodecZenohEncoding(packedId uint32, schema optional.Of[codec.Text[CodecZenohEncodingSchemaBound]]) CodecZenohEncoding {
	return CodecZenohEncoding{packedId: packedId, schema: schema}
}

// PackedId returns the element the value presents as PackedId.
func (s *CodecZenohEncoding) PackedId() uint32 {
	return s.packedId
}

// Schema returns the element the value presents as Schema.
func (s *CodecZenohEncoding) Schema() optional.Of[codec.Text[CodecZenohEncodingSchemaBound]] {
	return s.schema
}

// WireSchemaLen returns the schema_len the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecZenohEncoding) WireSchemaLen() optional.Of[uint64] {
	if _v, ok := s.Schema().Get(); ok {
		return optional.Some(uint64(_v.Len()))
	}
	return optional.None[uint64]()
}

// EncodeSource writes any source of CodecZenohEncoding into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives schema_len from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohEncodingSource, w codec.SceSink) error {
	var _dSchemaLen uint64
	if src.Schema().IsPresent() {
		_dSchemaLen = uint64(src.Schema().Proven().Len())
	}
	_cPackedId := src.PackedId()
	_cPackedId &^= 0x1
	if src.Schema().IsPresent() {
		_cPackedId |= 0x1
	}
	if err := codec.WriteVLEU32(w, uint32(_cPackedId)); err != nil {
		return err
	}
	if ((_cPackedId >> 0) & 1) != 0 {
		if err := codec.WriteVLEU64(w, uint64(_dSchemaLen)); err != nil {
			return err
		}
	}
	if src.Schema().IsPresent() {
		if err := w.WriteBytes([]byte(src.Schema().Proven().String())); err != nil {
			return err
		}
	}
	return nil
}

// DecodeCodecZenohEncoding decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohEncoding(cursor *codec.SceCursor) (*CodecZenohEncoding, error) {
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
	PackedId, err := cursor.ReadVLEU32()
	if err != nil { return nil, err }
	var SchemaLen optional.Of[uint64]
	if (PackedId & 0x00000001) != 0 {
		_v, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
		SchemaLen = optional.Some(_v)
	}
	var Schema optional.Of[string]
	if (PackedId & 0x00000001) != 0 {
		_len := SchemaLen.Proven()
		if _len > 255 {
			return nil, codec.ErrLengthOutOfRange
		}
		_n := int(_len)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		if !utf8.Valid(raw) {
			return nil, codec.ErrInvalidUTF8
		}
		Schema = optional.Some(string(raw))
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	_bSchema, err := codec.MakeOptionalText[CodecZenohEncodingSchemaBound](Schema)
	if err != nil {
		return nil, err
	}
	return &CodecZenohEncoding{
		packedId: PackedId,
		schema: _bSchema,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohEncoding) HasSchema() bool {
	return (s.packedId & 0x00000001) != 0
}

func (s *CodecZenohEncoding) SetHasSchema(v bool) {
	if v {
		s.packedId |= 0x00000001
	} else {
		s.packedId &^= 0x00000001
	}
}

// Encode writes the CodecZenohEncoding into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohEncoding) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohEncoding) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 269)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
