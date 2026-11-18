// SCE-MAP: codec_zenoh_response:76 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_response

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
	"unicode/utf8"
	"example.com/sce-forge/codec_zenoh_ext_entry"
	"example.com/sce-forge/codec_zenoh_reply"
	"example.com/sce-forge/codec_zenoh_err"
)

// CodecZenohResponseSuffixBound is the bound of the suffix element of CodecZenohResponse: 0 to 65535 UTF-8 bytes
// (docs/adr/0077 §3). A Text of it is made only within it.
type CodecZenohResponseSuffixBound struct{}

// Least is the fewest UTF-8 bytes the element holds.
func (CodecZenohResponseSuffixBound) Least() int { return 0 }

// Most is the most UTF-8 bytes the element holds.
func (CodecZenohResponseSuffixBound) Most() int { return 65535 }

// CodecZenohResponseExtensionsBound is the bound of the extensions element of CodecZenohResponse: 1 or more entries, the protocol stating no most
// (docs/adr/0065 §6 R2). A List of it is made only within it.
type CodecZenohResponseExtensionsBound struct{}

// Least is the fewest entries the element holds.
func (CodecZenohResponseExtensionsBound) Least() int { return 1 }

// Most is the most entries the element holds.
func (CodecZenohResponseExtensionsBound) Most() int { return codec.Unbounded }

// CodecZenohResponseUnknownTag is the tag the catch-all arm of CodecZenohResponse's variant holds: one
// the bits the tag is read from hold and no arm names, held to that where a
// value of it is made (docs/adr/0065 §6 R3, docs/adr/0081 §2), so the catch-all
// a value holds is written and read back as the catch-all. Its zero value
// holds 0x0, the least such tag.
type CodecZenohResponseUnknownTag struct {
	tag uint8
	set bool
}

// MakeCodecZenohResponseUnknownTag returns tag as the tag of the catch-all, and false where an
// arm names it or the bits the tag is read from do not hold it.
func MakeCodecZenohResponseUnknownTag(tag uint8) (CodecZenohResponseUnknownTag, bool) {
	if tag >= 0x4 && tag <= 0x5 || tag > 0x1F {
		return CodecZenohResponseUnknownTag{}, false
	}
	return CodecZenohResponseUnknownTag{tag: tag, set: true}, true
}

// Value returns the tag.
func (t CodecZenohResponseUnknownTag) Value() uint8 {
	if !t.set {
		return 0x0
	}
	return t.tag
}

// CodecZenohResponseVariant is the arm of the variant a CodecZenohResponse holds (docs/adr/0081 §2):
// one of CodecZenohResponseVariantCodecZenohReply, CodecZenohResponseVariantCodecZenohErr, CodecZenohResponseVariantDefault, which only this package declares.
type CodecZenohResponseVariant interface {
	isCodecZenohResponseVariant()
}

// CodecZenohResponseVariantCodecZenohReply is the arm of CodecZenohResponseVariant whose body is a CodecZenohReply, chosen by the tag 0x4.
type CodecZenohResponseVariantCodecZenohReply struct {
	body codec_zenoh_reply.CodecZenohReplySource
}

// MakeCodecZenohResponseVariantCodecZenohReply returns the arm that holds what it is handed.
func MakeCodecZenohResponseVariantCodecZenohReply(body codec_zenoh_reply.CodecZenohReplySource) CodecZenohResponseVariantCodecZenohReply {
	return CodecZenohResponseVariantCodecZenohReply{body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecZenohReply its codec makes by default.
func (a CodecZenohResponseVariantCodecZenohReply) Body() codec_zenoh_reply.CodecZenohReplySource {
	if a.body == nil {
		return codec_zenoh_reply.NewCodecZenohReply()
	}
	return a.body
}

func (CodecZenohResponseVariantCodecZenohReply) isCodecZenohResponseVariant() {}

// CodecZenohResponseVariantCodecZenohErr is the arm of CodecZenohResponseVariant whose body is a CodecZenohErr, chosen by the tag 0x5.
type CodecZenohResponseVariantCodecZenohErr struct {
	body codec_zenoh_err.CodecZenohErrSource
}

// MakeCodecZenohResponseVariantCodecZenohErr returns the arm that holds what it is handed.
func MakeCodecZenohResponseVariantCodecZenohErr(body codec_zenoh_err.CodecZenohErrSource) CodecZenohResponseVariantCodecZenohErr {
	return CodecZenohResponseVariantCodecZenohErr{body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecZenohErr its codec makes by default.
func (a CodecZenohResponseVariantCodecZenohErr) Body() codec_zenoh_err.CodecZenohErrSource {
	if a.body == nil {
		return codec_zenoh_err.NewCodecZenohErr()
	}
	return a.body
}

func (CodecZenohResponseVariantCodecZenohErr) isCodecZenohResponseVariant() {}

// CodecZenohResponseVariantDefault is the arm of CodecZenohResponseVariant whose body is a CodecZenohReply, chosen by every tag no arm names.
type CodecZenohResponseVariantDefault struct {
	tag  CodecZenohResponseUnknownTag
	body codec_zenoh_reply.CodecZenohReplySource
}

// MakeCodecZenohResponseVariantDefault returns the arm that holds what it is handed.
func MakeCodecZenohResponseVariantDefault(tag CodecZenohResponseUnknownTag, body codec_zenoh_reply.CodecZenohReplySource) CodecZenohResponseVariantDefault {
	return CodecZenohResponseVariantDefault{tag: tag, body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecZenohReply its codec makes by default.
func (a CodecZenohResponseVariantDefault) Body() codec_zenoh_reply.CodecZenohReplySource {
	if a.body == nil {
		return codec_zenoh_reply.NewCodecZenohReply()
	}
	return a.body
}

// Tag returns the tag the arm holds, which no arm names.
func (a CodecZenohResponseVariantDefault) Tag() CodecZenohResponseUnknownTag {
	return a.tag
}

func (CodecZenohResponseVariantDefault) isCodecZenohResponseVariant() {}

// CodecZenohResponseSource is what the encode of CodecZenohResponse reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohResponse is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohResponseSource interface {
	Header() uint8
	RequestId() uint64
	KeyId() uint32
	Suffix() optional.Of[codec.Text[CodecZenohResponseSuffixBound]]
	Extensions() optional.Of[codec.List[CodecZenohResponseExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]]
	Body() CodecZenohResponseVariant
}

// CodecZenohResponse represents the codec frame layout.
type CodecZenohResponse struct {
	header uint8
	requestId uint64
	keyId uint32
	suffix optional.Of[codec.Text[CodecZenohResponseSuffixBound]]
	extensions optional.Of[codec.List[CodecZenohResponseExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]]
	body CodecZenohResponseVariant
}

// MakeCodecZenohResponse returns the CodecZenohResponse that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohResponse, or as its zero value, and is read through its methods.
func MakeCodecZenohResponse(header uint8, requestId uint64, keyId uint32, suffix optional.Of[codec.Text[CodecZenohResponseSuffixBound]], extensions optional.Of[codec.List[CodecZenohResponseExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]], body CodecZenohResponseVariant) CodecZenohResponse {
	return CodecZenohResponse{header: header, requestId: requestId, keyId: keyId, suffix: suffix, extensions: extensions, body: body}
}

// Header returns the element the value presents as Header.
func (s *CodecZenohResponse) Header() uint8 {
	return s.header
}

// RequestId returns the element the value presents as RequestId.
func (s *CodecZenohResponse) RequestId() uint64 {
	return s.requestId
}

// KeyId returns the element the value presents as KeyId.
func (s *CodecZenohResponse) KeyId() uint32 {
	return s.keyId
}

// Suffix returns the element the value presents as Suffix.
func (s *CodecZenohResponse) Suffix() optional.Of[codec.Text[CodecZenohResponseSuffixBound]] {
	return s.suffix
}

// Extensions returns the element the value presents as Extensions.
func (s *CodecZenohResponse) Extensions() optional.Of[codec.List[CodecZenohResponseExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]] {
	return s.extensions
}

// Body returns the arm of the variant the value holds; a value that
// was handed none, as the zero value, holds CodecZenohResponseVariantCodecZenohReply{}.
func (s *CodecZenohResponse) Body() CodecZenohResponseVariant {
	if s.body == nil {
		return CodecZenohResponseVariantCodecZenohReply{}
	}
	return s.body
}

// WireSuffixLen returns the suffix_len the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecZenohResponse) WireSuffixLen() optional.Of[uint64] {
	if _v, ok := s.Suffix().Get(); ok {
		return optional.Some(uint64(_v.Len()))
	}
	return optional.None[uint64]()
}

// CodecZenohResponseOverrides holds the one-bit flags of CodecZenohResponse a parent may
// set in place of the ones a value presents, such as the flag a chain whose
// entries are CodecZenohResponse ends them on, which the chain derives from where
// each entry stands. A flag it leaves absent is the one the value presents.
type CodecZenohResponseOverrides struct {
	// HeaderM is the `M` flag of `header`, where a parent sets it.
	HeaderM optional.Of[bool]
}

// EncodeSource writes any source of CodecZenohResponse into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives suffix_len from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohResponseSource, w codec.SceSink) error {
	return EncodeSourceOverriding(src, w, CodecZenohResponseOverrides{})
}

// EncodeSourceOverriding is EncodeSource, each flag overrides sets written in
// place of the one src presents.
func EncodeSourceOverriding(src CodecZenohResponseSource, w codec.SceSink, overrides CodecZenohResponseOverrides) error {
	_rBody := src.Body()
	var _dSuffixLen uint64
	if src.Suffix().IsPresent() {
		_dSuffixLen = uint64(src.Suffix().Proven().Len())
	}
	_cHeader := src.Header()
	_cHeader &^= 0x20
	if src.Suffix().IsPresent() {
		_cHeader |= 0x20
	}
	_cHeader &^= 0x80
	if src.Extensions().IsPresent() {
		_cHeader |= 0x80
	}
	if _o, ok := overrides.HeaderM.Get(); ok {
		_cHeader &^= 0x40
		if _o {
			_cHeader |= 0x40
		}
	}
	if err := w.WriteBytes([]byte{byte(_cHeader)}); err != nil {
		return err
	}
	if err := codec.WriteVLEU64(w, uint64(src.RequestId())); err != nil {
		return err
	}
	if err := codec.WriteVLEU32(w, uint32(src.KeyId())); err != nil {
		return err
	}
	if ((_cHeader >> 5) & 1) != 0 {
		if err := codec.WriteVLEU64(w, uint64(_dSuffixLen)); err != nil {
			return err
		}
	}
	if src.Suffix().IsPresent() {
		if err := w.WriteBytes([]byte(src.Suffix().Proven().String())); err != nil {
			return err
		}
	}
	if src.Extensions().IsPresent() {
		{
			_leftExtensions := src.Extensions().Proven().Len()
			for _e := range src.Extensions().Proven().All() {
				_leftExtensions--
				if err := codec_zenoh_ext_entry.EncodeSourceOverriding(_e, w, codec_zenoh_ext_entry.CodecZenohExtEntryOverrides{HeaderZ: optional.Some(_leftExtensions != 0)}); err != nil {
					return err
				}
			}
		}
	}
	switch _b := _rBody.(type) {
	case CodecZenohResponseVariantCodecZenohReply:
		if err := codec_zenoh_reply.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	case CodecZenohResponseVariantCodecZenohErr:
		if err := codec_zenoh_err.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	case CodecZenohResponseVariantDefault:
		if err := codec_zenoh_reply.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	default:
		panic("CodecZenohResponse: the source presents no arm of its variant")
	}
	return nil
}

// NewCodecZenohResponse returns a CodecZenohResponse initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecZenohResponse().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecZenohResponse{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecZenohResponse() *CodecZenohResponse {
	return &CodecZenohResponse{
		header: uint8(0x1b),
	}
}

// DecodeCodecZenohResponse decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohResponse(cursor *codec.SceCursor) (*CodecZenohResponse, error) {
	// RFC §synth-5-B peek-byte / streaming-prefix:
	// streaming prefix decode (variable-length fields supported via
	// per-field present_if/tlv-chain/embed/repeat helpers). Peek-byte
	// mode additionally peeks the cursor's next byte for variant tag
	// without advancing — arm body decoder reads it as own header.
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
	RequestId, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	KeyId, err := cursor.ReadVLEU32()
	if err != nil { return nil, err }
	var SuffixLen optional.Of[uint64]
	if (Header & 0x20) != 0 {
		_v, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
		SuffixLen = optional.Some(_v)
	}
	var Suffix optional.Of[string]
	if (Header & 0x20) != 0 {
		_len := SuffixLen.Proven()
		if _len > 65535 {
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
		Suffix = optional.Some(string(raw))
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	var Extensions optional.Of[[]codec_zenoh_ext_entry.CodecZenohExtEntry]
	if (Header & 0x80) != 0 {
		_list := make([]codec_zenoh_ext_entry.CodecZenohExtEntry, 0)
		_more := false
		for {
			_elem, err := codec_zenoh_ext_entry.DecodeCodecZenohExtEntry(cursor)
			if err != nil {
				return nil, err
			}
			_more = _elem.Z()
			_list = append(_list, *_elem)
			if !_more {
				break
			}
		}
		if _more && cursor.Remaining() == 0 {
			return nil, codec.ErrNeedMoreBytes
		}
		Extensions = optional.Some(_list)
	}
	_peekSlice, err := cursor.PeekSlice(1)
	if err != nil {
		return nil, err
	}
	_peek := _peekSlice[0]
	_bSuffix, err := codec.MakeOptionalText[CodecZenohResponseSuffixBound](Suffix)
	if err != nil {
		return nil, err
	}
	_lExtensions, err := codec.MakeOptionalList[CodecZenohResponseExtensionsBound](codec.OptionalSourcesOf(Extensions, func(e *codec_zenoh_ext_entry.CodecZenohExtEntry) codec_zenoh_ext_entry.CodecZenohExtEntrySource { return e }))
	if err != nil {
		return nil, err
	}
	// Dispatch on the tag field; each arm decodes its body codec from
	// the cursor. The default arm (when declared) carries the runtime
	// tag value so encode can round-trip it back onto the wire.
	var body CodecZenohResponseVariant
	switch uint8((_peek >> 0) & 0x1F) {
	case 4:
		_arm, err := codec_zenoh_reply.DecodeCodecZenohReply(cursor)
		if err != nil {
			return nil, err
		}
		body = MakeCodecZenohResponseVariantCodecZenohReply(_arm)
	case 5:
		_arm, err := codec_zenoh_err.DecodeCodecZenohErr(cursor)
		if err != nil {
			return nil, err
		}
		body = MakeCodecZenohResponseVariantCodecZenohErr(_arm)
	default:
		_arm, err := codec_zenoh_reply.DecodeCodecZenohReply(cursor)
		if err != nil {
			return nil, err
		}
		_tag := uint8((_peek >> 0) & 0x1F)
		body = MakeCodecZenohResponseVariantDefault(CodecZenohResponseUnknownTag{tag: _tag, set: true}, _arm)
	}
	return &CodecZenohResponse{
		header: Header,
		requestId: RequestId,
		keyId: KeyId,
		suffix: _bSuffix,
		extensions: _lExtensions,
		body: body,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohResponse) Mid() uint8 {
	return uint8((s.header >> 0) & 0x1F)
}

func (s *CodecZenohResponse) SetMid(v uint8) {
	const _shiftedMask uint8 = 0x1F << 0
	_val := (uint8(v) & 0x1F) << 0
	s.header = (s.header &^ _shiftedMask) | _val
}

func (s *CodecZenohResponse) N() bool {
	return (s.header & 0x20) != 0
}

func (s *CodecZenohResponse) SetN(v bool) {
	if v {
		s.header |= 0x20
	} else {
		s.header &^= 0x20
	}
}

func (s *CodecZenohResponse) M() bool {
	return (s.header & 0x40) != 0
}

func (s *CodecZenohResponse) SetM(v bool) {
	if v {
		s.header |= 0x40
	} else {
		s.header &^= 0x40
	}
}

func (s *CodecZenohResponse) Z() bool {
	return (s.header & 0x80) != 0
}

func (s *CodecZenohResponse) SetZ(v bool) {
	if v {
		s.header |= 0x80
	} else {
		s.header &^= 0x80
	}
}

// Encode writes the CodecZenohResponse into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohResponse) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohResponse) EncodeToBytes() []byte {
	_dst := make([]byte, 0)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
