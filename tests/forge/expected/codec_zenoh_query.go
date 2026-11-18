// SCE-MAP: codec_zenoh_query:52 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_query

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
	"example.com/sce-forge/codec_zenoh_ext_entry"
)

// CodecZenohQueryParametersBound is the bound of the parameters element of CodecZenohQuery: 0 or more bytes, the protocol stating no most
// (docs/adr/0077 §3). A Bytes of it is made only within it.
type CodecZenohQueryParametersBound struct{}

// Least is the fewest bytes the element holds.
func (CodecZenohQueryParametersBound) Least() int { return 0 }

// Most is the most bytes the element holds.
func (CodecZenohQueryParametersBound) Most() int { return codec.Unbounded }

// CodecZenohQueryExtensionsBound is the bound of the extensions element of CodecZenohQuery: 0 or more entries, the protocol stating no most
// (docs/adr/0065 §6 R2). A List of it is made only within it.
type CodecZenohQueryExtensionsBound struct{}

// Least is the fewest entries the element holds.
func (CodecZenohQueryExtensionsBound) Least() int { return 0 }

// Most is the most entries the element holds.
func (CodecZenohQueryExtensionsBound) Most() int { return codec.Unbounded }

// CodecZenohQuerySource is what the encode of CodecZenohQuery reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohQuery is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohQuerySource interface {
	Header() uint8
	Consolidation() optional.Of[uint8]
	Parameters() optional.Of[codec.Bytes[CodecZenohQueryParametersBound]]
	Extensions() optional.Of[codec.List[CodecZenohQueryExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]]
}

// CodecZenohQuery represents the codec frame layout.
type CodecZenohQuery struct {
	header uint8
	consolidation optional.Of[uint8]
	parameters optional.Of[codec.Bytes[CodecZenohQueryParametersBound]]
	extensions optional.Of[codec.List[CodecZenohQueryExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]]
}

// MakeCodecZenohQuery returns the CodecZenohQuery that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohQuery, or as its zero value, and is read through its methods.
func MakeCodecZenohQuery(header uint8, consolidation optional.Of[uint8], parameters optional.Of[codec.Bytes[CodecZenohQueryParametersBound]], extensions optional.Of[codec.List[CodecZenohQueryExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]]) CodecZenohQuery {
	return CodecZenohQuery{header: header, consolidation: consolidation, parameters: parameters, extensions: extensions}
}

// Header returns the element the value presents as Header.
func (s *CodecZenohQuery) Header() uint8 {
	return s.header
}

// Consolidation returns the element the value presents as Consolidation.
func (s *CodecZenohQuery) Consolidation() optional.Of[uint8] {
	return s.consolidation
}

// Parameters returns the element the value presents as Parameters.
func (s *CodecZenohQuery) Parameters() optional.Of[codec.Bytes[CodecZenohQueryParametersBound]] {
	return s.parameters
}

// Extensions returns the element the value presents as Extensions.
func (s *CodecZenohQuery) Extensions() optional.Of[codec.List[CodecZenohQueryExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]] {
	return s.extensions
}

// WireParametersLen returns the parameters_len the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecZenohQuery) WireParametersLen() optional.Of[uint64] {
	if _v, ok := s.Parameters().Get(); ok {
		return optional.Some(uint64(_v.Len()))
	}
	return optional.None[uint64]()
}

// EncodeSource writes any source of CodecZenohQuery into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives parameters_len from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohQuerySource, w codec.SceSink) error {
	var _dParametersLen uint64
	if src.Parameters().IsPresent() {
		_dParametersLen = uint64(src.Parameters().Proven().Len())
	}
	_cHeader := src.Header()
	_cHeader &^= 0x20
	if src.Consolidation().IsPresent() {
		_cHeader |= 0x20
	}
	_cHeader &^= 0x40
	if src.Parameters().IsPresent() {
		_cHeader |= 0x40
	}
	_cHeader &^= 0x80
	if src.Extensions().IsPresent() {
		_cHeader |= 0x80
	}
	if err := w.WriteBytes([]byte{byte(_cHeader)}); err != nil {
		return err
	}
	if src.Consolidation().IsPresent() {
		if err := w.WriteBytes([]byte{byte(src.Consolidation().Proven())}); err != nil {
			return err
		}
	}
	if ((_cHeader >> 6) & 1) != 0 {
		if err := codec.WriteVLEU64(w, uint64(_dParametersLen)); err != nil {
			return err
		}
	}
	if src.Parameters().IsPresent() {
		if err := w.WriteBytes(src.Parameters().Proven().Bytes()); err != nil {
			return err
		}
	}
	if src.Extensions().IsPresent() {
		for _e := range src.Extensions().Proven().All() {
			if err := codec_zenoh_ext_entry.EncodeSource(_e, w); err != nil {
				return err
			}
		}
	}
	return nil
}

// NewCodecZenohQuery returns a CodecZenohQuery initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecZenohQuery().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecZenohQuery{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecZenohQuery() *CodecZenohQuery {
	return &CodecZenohQuery{
		header: uint8(0x03),
	}
}

// DecodeCodecZenohQuery decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohQuery(cursor *codec.SceCursor) (*CodecZenohQuery, error) {
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
	var Header uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Header = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var Consolidation optional.Of[uint8]
	if (Header & 0x20) != 0 {
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		_v := raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
		Consolidation = optional.Some(_v)
	}
	var ParametersLen optional.Of[uint64]
	if (Header & 0x40) != 0 {
		_v, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
		ParametersLen = optional.Some(_v)
	}
	var Parameters optional.Of[[]byte]
	if (Header & 0x40) != 0 {
		_len := ParametersLen.Proven()
		if uint64(_len) > uint64(^uint(0)>>1)-0 {
			return nil, codec.ErrNeedMoreBytes
		}
		_n := int(_len)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Parameters = optional.Some(append([]byte(nil), raw...))
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	var Extensions optional.Of[[]codec_zenoh_ext_entry.CodecZenohExtEntry]
	if (Header & 0x80) != 0 {
		_list := make([]codec_zenoh_ext_entry.CodecZenohExtEntry, 0)
		for {
			if cursor.Remaining() == 0 {
				break
			}
			_elem, err := codec_zenoh_ext_entry.DecodeCodecZenohExtEntry(cursor)
			if err != nil {
				return nil, err
			}
			_list = append(_list, *_elem)
		}
		Extensions = optional.Some(_list)
	}
	_bParameters, err := codec.MakeOptionalBytes[CodecZenohQueryParametersBound](Parameters)
	if err != nil {
		return nil, err
	}
	_lExtensions, err := codec.MakeOptionalList[CodecZenohQueryExtensionsBound](codec.OptionalSourcesOf(Extensions, func(e *codec_zenoh_ext_entry.CodecZenohExtEntry) codec_zenoh_ext_entry.CodecZenohExtEntrySource { return e }))
	if err != nil {
		return nil, err
	}
	return &CodecZenohQuery{
		header: Header,
		consolidation: Consolidation,
		parameters: _bParameters,
		extensions: _lExtensions,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohQuery) Mid() uint8 {
	return uint8((s.header >> 0) & 0x1F)
}

func (s *CodecZenohQuery) SetMid(v uint8) {
	const _shiftedMask uint8 = 0x1F << 0
	_val := (uint8(v) & 0x1F) << 0
	s.header = (s.header &^ _shiftedMask) | _val
}

func (s *CodecZenohQuery) C() bool {
	return (s.header & 0x20) != 0
}

func (s *CodecZenohQuery) SetC(v bool) {
	if v {
		s.header |= 0x20
	} else {
		s.header &^= 0x20
	}
}

func (s *CodecZenohQuery) P() bool {
	return (s.header & 0x40) != 0
}

func (s *CodecZenohQuery) SetP(v bool) {
	if v {
		s.header |= 0x40
	} else {
		s.header &^= 0x40
	}
}

func (s *CodecZenohQuery) Z() bool {
	return (s.header & 0x80) != 0
}

func (s *CodecZenohQuery) SetZ(v bool) {
	if v {
		s.header |= 0x80
	} else {
		s.header &^= 0x80
	}
}

// Encode writes the CodecZenohQuery into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohQuery) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohQuery) EncodeToBytes() []byte {
	_dst := make([]byte, 0)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
