// SCE-MAP: codec_ext_encoding_info:44 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_ext_encoding_info

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
)

// CodecExtEncodingInfoSchemaBound is the bound of the schema element of CodecExtEncodingInfo: 0 to 64 bytes
// (docs/adr/0077 §3). A Bytes of it is made only within it.
type CodecExtEncodingInfoSchemaBound struct{}

// Least is the fewest bytes the element holds.
func (CodecExtEncodingInfoSchemaBound) Least() int { return 0 }

// Most is the most bytes the element holds.
func (CodecExtEncodingInfoSchemaBound) Most() int { return 64 }

// CodecExtEncodingInfoCombinedIdHasSchemaHasSchema is the CombinedIdHasSchema group of CodecExtEncodingInfo where `combined_id.has_schema` holds (docs/adr/0065 §6 R4): the
// members present then.
type CodecExtEncodingInfoCombinedIdHasSchemaHasSchema struct {
	schema codec.Bytes[CodecExtEncodingInfoSchemaBound]
}

// MakeCodecExtEncodingInfoCombinedIdHasSchemaHasSchema returns the arm that holds each member it is handed.
func MakeCodecExtEncodingInfoCombinedIdHasSchemaHasSchema(schema codec.Bytes[CodecExtEncodingInfoSchemaBound]) CodecExtEncodingInfoCombinedIdHasSchemaHasSchema {
	return CodecExtEncodingInfoCombinedIdHasSchemaHasSchema{schema: schema}
}

// Schema returns the member the arm presents as Schema.
func (a CodecExtEncodingInfoCombinedIdHasSchemaHasSchema) Schema() codec.Bytes[CodecExtEncodingInfoSchemaBound] {
	return a.schema
}

func (CodecExtEncodingInfoCombinedIdHasSchemaHasSchema) isCodecExtEncodingInfoCombinedIdHasSchema() {}

// CodecExtEncodingInfoCombinedIdHasSchemaNotHasSchema is the CombinedIdHasSchema group of CodecExtEncodingInfo where `combined_id.has_schema` does not hold (docs/adr/0065 §6 R4): the
// members present then.
type CodecExtEncodingInfoCombinedIdHasSchemaNotHasSchema struct {
	schemaSize uint8
}

// MakeCodecExtEncodingInfoCombinedIdHasSchemaNotHasSchema returns the arm that holds each member it is handed.
func MakeCodecExtEncodingInfoCombinedIdHasSchemaNotHasSchema(schemaSize uint8) CodecExtEncodingInfoCombinedIdHasSchemaNotHasSchema {
	return CodecExtEncodingInfoCombinedIdHasSchemaNotHasSchema{schemaSize: schemaSize}
}

// SchemaSize returns the member the arm presents as SchemaSize.
func (a CodecExtEncodingInfoCombinedIdHasSchemaNotHasSchema) SchemaSize() uint8 {
	return a.schemaSize
}

func (CodecExtEncodingInfoCombinedIdHasSchemaNotHasSchema) isCodecExtEncodingInfoCombinedIdHasSchema() {}

// CodecExtEncodingInfoCombinedIdHasSchema is the CombinedIdHasSchema group of CodecExtEncodingInfo: the arm the value holds, each the
// members present while the gates hold its values (docs/adr/0065 §6 R4),
// one of CodecExtEncodingInfoCombinedIdHasSchemaHasSchema, CodecExtEncodingInfoCombinedIdHasSchemaNotHasSchema, which only this package declares.
type CodecExtEncodingInfoCombinedIdHasSchema interface {
	isCodecExtEncodingInfoCombinedIdHasSchema()
}

// CodecExtEncodingInfoSource is what the encode of CodecExtEncodingInfo reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecExtEncodingInfo is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecExtEncodingInfoSource interface {
	CombinedId() uint32
	CombinedIdHasSchema() CodecExtEncodingInfoCombinedIdHasSchema
}

// CodecExtEncodingInfo represents the codec frame layout.
type CodecExtEncodingInfo struct {
	combinedId uint32
	combinedIdHasSchema CodecExtEncodingInfoCombinedIdHasSchema
}

// MakeCodecExtEncodingInfo returns the CodecExtEncodingInfo that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecExtEncodingInfo, or as its zero value, and is read through its methods.
func MakeCodecExtEncodingInfo(combinedId uint32, combinedIdHasSchema CodecExtEncodingInfoCombinedIdHasSchema) CodecExtEncodingInfo {
	return CodecExtEncodingInfo{combinedId: combinedId, combinedIdHasSchema: combinedIdHasSchema}
}

// CombinedId returns the element the value presents as CombinedId.
func (s *CodecExtEncodingInfo) CombinedId() uint32 {
	return s.combinedId
}

// CombinedIdHasSchema returns the element the value presents as CombinedIdHasSchema.
func (s *CodecExtEncodingInfo) CombinedIdHasSchema() CodecExtEncodingInfoCombinedIdHasSchema {
	if s.combinedIdHasSchema == nil {
		return CodecExtEncodingInfoCombinedIdHasSchemaHasSchema{}
	}
	return s.combinedIdHasSchema
}

// WireSchemaSize returns the schema_size the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecExtEncodingInfo) WireSchemaSize() uint8 {
	_rCombinedIdHasSchema := s.CombinedIdHasSchema()
	var _mSchema optional.Of[codec.Bytes[CodecExtEncodingInfoSchemaBound]]
	var _mSchemaSize optional.Of[uint8]
	switch _g := _rCombinedIdHasSchema.(type) {
	case CodecExtEncodingInfoCombinedIdHasSchemaHasSchema:
		_mSchema = optional.Some(_g.Schema())
	case CodecExtEncodingInfoCombinedIdHasSchemaNotHasSchema:
		_mSchemaSize = optional.Some(_g.SchemaSize())
	}
	if _v, ok := _mSchema.Get(); ok {
		return uint8(_v.Len())
	}
	if _s, ok := _mSchemaSize.Get(); ok {
		return _s
	}
	return 0
}

// EncodeSource writes any source of CodecExtEncodingInfo into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives schema_size from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecExtEncodingInfoSource, w codec.SceSink) error {
	_rCombinedIdHasSchema := src.CombinedIdHasSchema()
	var _mSchema optional.Of[codec.Bytes[CodecExtEncodingInfoSchemaBound]]
	var _mSchemaSize optional.Of[uint8]
	switch _g := _rCombinedIdHasSchema.(type) {
	case CodecExtEncodingInfoCombinedIdHasSchemaHasSchema:
		_mSchema = optional.Some(_g.Schema())
	case CodecExtEncodingInfoCombinedIdHasSchemaNotHasSchema:
		_mSchemaSize = optional.Some(_g.SchemaSize())
	}
	var _dSchemaSize uint8
	if _mSchema.IsPresent() {
		_dSchemaSize = uint8(_mSchema.Proven().Len())
	} else if _s, ok := _mSchemaSize.Get(); ok {
		_dSchemaSize = _s
	}
	_cCombinedId := src.CombinedId()
	_cCombinedId &^= 0x1
	if _mSchema.IsPresent() {
		_cCombinedId |= 0x1
	}
	if err := codec.WriteVLEU32(w, uint32(_cCombinedId)); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{byte(_dSchemaSize)}); err != nil {
		return err
	}
	if _mSchema.IsPresent() {
		if err := w.WriteBytes(_mSchema.Proven().Bytes()); err != nil {
			return err
		}
	}
	return nil
}

// DecodeCodecExtEncodingInfo decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecExtEncodingInfo(cursor *codec.SceCursor) (*CodecExtEncodingInfo, error) {
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
	CombinedId, err := cursor.ReadVLEU32()
	if err != nil { return nil, err }
	var SchemaSize uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		SchemaSize = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var Schema optional.Of[[]byte]
	if (CombinedId & 0x00000001) != 0 {
		_len := SchemaSize
		if _len > 64 {
			return nil, codec.ErrLengthOutOfRange
		}
		_n := int(_len)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Schema = optional.Some(append([]byte(nil), raw...))
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	_bSchema, err := codec.MakeOptionalBytes[CodecExtEncodingInfoSchemaBound](Schema)
	if err != nil {
		return nil, err
	}
	var _gCombinedIdHasSchema CodecExtEncodingInfoCombinedIdHasSchema
	if ((CombinedId >> 0) & 1) != 0 {
		_gCombinedIdHasSchema = MakeCodecExtEncodingInfoCombinedIdHasSchemaHasSchema(_bSchema.Proven())
	} else {
		_gCombinedIdHasSchema = MakeCodecExtEncodingInfoCombinedIdHasSchemaNotHasSchema(SchemaSize)
	}
	return &CodecExtEncodingInfo{
		combinedId: CombinedId,
		combinedIdHasSchema: _gCombinedIdHasSchema,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecExtEncodingInfo) HasSchema() bool {
	return (s.combinedId & 0x00000001) != 0
}

func (s *CodecExtEncodingInfo) SetHasSchema(v bool) {
	if v {
		s.combinedId |= 0x00000001
	} else {
		s.combinedId &^= 0x00000001
	}
}

// Encode writes the CodecExtEncodingInfo into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecExtEncodingInfo) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecExtEncodingInfo) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 71)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
