// SCE-MAP: codec_zenoh_oam:56 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_oam

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
	"example.com/sce-forge/codec_zenoh_ext_entry"
	"example.com/sce-forge/codec_zenoh_ext_unit"
	"example.com/sce-forge/codec_zenoh_ext_zint"
	"example.com/sce-forge/codec_zenoh_ext_zbuf"
)

// CodecZenohOamExtensionsBound is the bound of the extensions element of CodecZenohOam: 1 or more entries, the protocol stating no most
// (docs/adr/0065 §6 R2). A List of it is made only within it.
type CodecZenohOamExtensionsBound struct{}

// Least is the fewest entries the element holds.
func (CodecZenohOamExtensionsBound) Least() int { return 1 }

// Most is the most entries the element holds.
func (CodecZenohOamExtensionsBound) Most() int { return codec.Unbounded }

// CodecZenohOamUnknownTag is the tag the catch-all arm of CodecZenohOam's variant holds: one
// the bits the tag is read from hold and no arm names, held to that where a
// value of it is made (docs/adr/0065 §6 R3, docs/adr/0081 §2), so the catch-all
// a value holds is written and read back as the catch-all. Its zero value
// holds 0x3, the least such tag.
type CodecZenohOamUnknownTag struct {
	tag uint8
	set bool
}

// MakeCodecZenohOamUnknownTag returns tag as the tag of the catch-all, and false where an
// arm names it or the bits the tag is read from do not hold it.
func MakeCodecZenohOamUnknownTag(tag uint8) (CodecZenohOamUnknownTag, bool) {
	if tag <= 0x2 || tag > 0x3 {
		return CodecZenohOamUnknownTag{}, false
	}
	return CodecZenohOamUnknownTag{tag: tag, set: true}, true
}

// Value returns the tag.
func (t CodecZenohOamUnknownTag) Value() uint8 {
	if !t.set {
		return 0x3
	}
	return t.tag
}

// CodecZenohOamVariant is the arm of the variant a CodecZenohOam holds (docs/adr/0081 §2):
// one of CodecZenohOamVariantCodecZenohExtUnit, CodecZenohOamVariantCodecZenohExtZint, CodecZenohOamVariantCodecZenohExtZbuf, CodecZenohOamVariantDefault, which only this package declares.
type CodecZenohOamVariant interface {
	isCodecZenohOamVariant()
}

// CodecZenohOamVariantCodecZenohExtUnit is the arm of CodecZenohOamVariant whose body is a CodecZenohExtUnit, chosen by the tag 0x0.
type CodecZenohOamVariantCodecZenohExtUnit struct {
	body codec_zenoh_ext_unit.CodecZenohExtUnitSource
}

// MakeCodecZenohOamVariantCodecZenohExtUnit returns the arm that holds what it is handed.
func MakeCodecZenohOamVariantCodecZenohExtUnit(body codec_zenoh_ext_unit.CodecZenohExtUnitSource) CodecZenohOamVariantCodecZenohExtUnit {
	return CodecZenohOamVariantCodecZenohExtUnit{body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecZenohExtUnit its codec makes by default.
func (a CodecZenohOamVariantCodecZenohExtUnit) Body() codec_zenoh_ext_unit.CodecZenohExtUnitSource {
	if a.body == nil {
		return &codec_zenoh_ext_unit.CodecZenohExtUnit{}
	}
	return a.body
}

func (CodecZenohOamVariantCodecZenohExtUnit) isCodecZenohOamVariant() {}

// CodecZenohOamVariantCodecZenohExtZint is the arm of CodecZenohOamVariant whose body is a CodecZenohExtZint, chosen by the tag 0x1.
type CodecZenohOamVariantCodecZenohExtZint struct {
	body codec_zenoh_ext_zint.CodecZenohExtZintSource
}

// MakeCodecZenohOamVariantCodecZenohExtZint returns the arm that holds what it is handed.
func MakeCodecZenohOamVariantCodecZenohExtZint(body codec_zenoh_ext_zint.CodecZenohExtZintSource) CodecZenohOamVariantCodecZenohExtZint {
	return CodecZenohOamVariantCodecZenohExtZint{body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecZenohExtZint its codec makes by default.
func (a CodecZenohOamVariantCodecZenohExtZint) Body() codec_zenoh_ext_zint.CodecZenohExtZintSource {
	if a.body == nil {
		return &codec_zenoh_ext_zint.CodecZenohExtZint{}
	}
	return a.body
}

func (CodecZenohOamVariantCodecZenohExtZint) isCodecZenohOamVariant() {}

// CodecZenohOamVariantCodecZenohExtZbuf is the arm of CodecZenohOamVariant whose body is a CodecZenohExtZbuf, chosen by the tag 0x2.
type CodecZenohOamVariantCodecZenohExtZbuf struct {
	body codec_zenoh_ext_zbuf.CodecZenohExtZbufSource
}

// MakeCodecZenohOamVariantCodecZenohExtZbuf returns the arm that holds what it is handed.
func MakeCodecZenohOamVariantCodecZenohExtZbuf(body codec_zenoh_ext_zbuf.CodecZenohExtZbufSource) CodecZenohOamVariantCodecZenohExtZbuf {
	return CodecZenohOamVariantCodecZenohExtZbuf{body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecZenohExtZbuf its codec makes by default.
func (a CodecZenohOamVariantCodecZenohExtZbuf) Body() codec_zenoh_ext_zbuf.CodecZenohExtZbufSource {
	if a.body == nil {
		return &codec_zenoh_ext_zbuf.CodecZenohExtZbuf{}
	}
	return a.body
}

func (CodecZenohOamVariantCodecZenohExtZbuf) isCodecZenohOamVariant() {}

// CodecZenohOamVariantDefault is the arm of CodecZenohOamVariant whose body is a CodecZenohExtUnit, chosen by every tag no arm names.
type CodecZenohOamVariantDefault struct {
	tag  CodecZenohOamUnknownTag
	body codec_zenoh_ext_unit.CodecZenohExtUnitSource
}

// MakeCodecZenohOamVariantDefault returns the arm that holds what it is handed.
func MakeCodecZenohOamVariantDefault(tag CodecZenohOamUnknownTag, body codec_zenoh_ext_unit.CodecZenohExtUnitSource) CodecZenohOamVariantDefault {
	return CodecZenohOamVariantDefault{tag: tag, body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecZenohExtUnit its codec makes by default.
func (a CodecZenohOamVariantDefault) Body() codec_zenoh_ext_unit.CodecZenohExtUnitSource {
	if a.body == nil {
		return &codec_zenoh_ext_unit.CodecZenohExtUnit{}
	}
	return a.body
}

// Tag returns the tag the arm holds, which no arm names.
func (a CodecZenohOamVariantDefault) Tag() CodecZenohOamUnknownTag {
	return a.tag
}

func (CodecZenohOamVariantDefault) isCodecZenohOamVariant() {}

// CodecZenohOamSource is what the encode of CodecZenohOam reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohOam is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohOamSource interface {
	Header() uint8
	Id() uint16
	Extensions() optional.Of[codec.List[CodecZenohOamExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]]
	Body() CodecZenohOamVariant
}

// CodecZenohOam represents the codec frame layout.
type CodecZenohOam struct {
	header uint8
	id uint16
	extensions optional.Of[codec.List[CodecZenohOamExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]]
	body CodecZenohOamVariant
}

// MakeCodecZenohOam returns the CodecZenohOam that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohOam, or as its zero value, and is read through its methods.
func MakeCodecZenohOam(header uint8, id uint16, extensions optional.Of[codec.List[CodecZenohOamExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]], body CodecZenohOamVariant) CodecZenohOam {
	return CodecZenohOam{header: header, id: id, extensions: extensions, body: body}
}

// Header returns the element the value presents as Header.
func (s *CodecZenohOam) Header() uint8 {
	return s.header
}

// Id returns the element the value presents as Id.
func (s *CodecZenohOam) Id() uint16 {
	return s.id
}

// Extensions returns the element the value presents as Extensions.
func (s *CodecZenohOam) Extensions() optional.Of[codec.List[CodecZenohOamExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]] {
	return s.extensions
}

// Body returns the arm of the variant the value holds; a value that
// was handed none, as the zero value, holds CodecZenohOamVariantCodecZenohExtUnit{}.
func (s *CodecZenohOam) Body() CodecZenohOamVariant {
	if s.body == nil {
		return CodecZenohOamVariantCodecZenohExtUnit{}
	}
	return s.body
}

// EncodeSource writes any source of CodecZenohOam into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohOamSource, w codec.SceSink) error {
	_rBody := src.Body()
	var _tBody uint64
	switch _b := _rBody.(type) {
	case CodecZenohOamVariantCodecZenohExtUnit:
		_tBody = 0x0
	case CodecZenohOamVariantCodecZenohExtZint:
		_tBody = 0x1
	case CodecZenohOamVariantCodecZenohExtZbuf:
		_tBody = 0x2
	case CodecZenohOamVariantDefault:
		_tBody = uint64(_b.Tag().Value())
	}
	_cHeader := src.Header()
	_cHeader = (_cHeader &^ 0x60) | (uint8(_tBody & 0x3) << 5)
	_cHeader &^= 0x80
	if src.Extensions().IsPresent() {
		_cHeader |= 0x80
	}
	if err := w.WriteBytes([]byte{byte(_cHeader)}); err != nil {
		return err
	}
	if err := codec.WriteVLEU16(w, uint16(src.Id())); err != nil {
		return err
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
	case CodecZenohOamVariantCodecZenohExtUnit:
		if err := codec_zenoh_ext_unit.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	case CodecZenohOamVariantCodecZenohExtZint:
		if err := codec_zenoh_ext_zint.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	case CodecZenohOamVariantCodecZenohExtZbuf:
		if err := codec_zenoh_ext_zbuf.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	case CodecZenohOamVariantDefault:
		if err := codec_zenoh_ext_unit.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	default:
		panic("CodecZenohOam: the source presents no arm of its variant")
	}
	return nil
}

// NewCodecZenohOam returns a CodecZenohOam initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecZenohOam().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecZenohOam{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecZenohOam() *CodecZenohOam {
	return &CodecZenohOam{
		header: uint8(0x1f),
	}
}

// DecodeCodecZenohOam decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohOam(cursor *codec.SceCursor) (*CodecZenohOam, error) {
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
	Id, err := cursor.ReadVLEU16()
	if err != nil { return nil, err }
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
	_lExtensions, err := codec.MakeOptionalList[CodecZenohOamExtensionsBound](codec.OptionalSourcesOf(Extensions, func(e *codec_zenoh_ext_entry.CodecZenohExtEntry) codec_zenoh_ext_entry.CodecZenohExtEntrySource { return e }))
	if err != nil {
		return nil, err
	}
	// Dispatch on the tag field; each arm decodes its body codec from
	// the cursor. The default arm (when declared) carries the runtime
	// tag value so encode can round-trip it back onto the wire.
	var body CodecZenohOamVariant
	switch uint8((Header >> 5) & 0x03) {
	case 0:
		_arm, err := codec_zenoh_ext_unit.DecodeCodecZenohExtUnit(cursor)
		if err != nil {
			return nil, err
		}
		body = MakeCodecZenohOamVariantCodecZenohExtUnit(_arm)
	case 1:
		_arm, err := codec_zenoh_ext_zint.DecodeCodecZenohExtZint(cursor)
		if err != nil {
			return nil, err
		}
		body = MakeCodecZenohOamVariantCodecZenohExtZint(_arm)
	case 2:
		_arm, err := codec_zenoh_ext_zbuf.DecodeCodecZenohExtZbuf(cursor)
		if err != nil {
			return nil, err
		}
		body = MakeCodecZenohOamVariantCodecZenohExtZbuf(_arm)
	default:
		_arm, err := codec_zenoh_ext_unit.DecodeCodecZenohExtUnit(cursor)
		if err != nil {
			return nil, err
		}
		_tag := uint8((Header >> 5) & 0x03)
		body = MakeCodecZenohOamVariantDefault(CodecZenohOamUnknownTag{tag: _tag, set: true}, _arm)
	}
	return &CodecZenohOam{
		header: Header,
		id: Id,
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
func (s *CodecZenohOam) Mid() uint8 {
	return uint8((s.header >> 0) & 0x1F)
}

func (s *CodecZenohOam) SetMid(v uint8) {
	const _shiftedMask uint8 = 0x1F << 0
	_val := (uint8(v) & 0x1F) << 0
	s.header = (s.header &^ _shiftedMask) | _val
}

func (s *CodecZenohOam) Enc() uint8 {
	return uint8((s.header >> 5) & 0x03)
}

func (s *CodecZenohOam) SetEnc(v uint8) {
	const _shiftedMask uint8 = 0x03 << 5
	_val := (uint8(v) & 0x03) << 5
	s.header = (s.header &^ _shiftedMask) | _val
}

func (s *CodecZenohOam) Z() bool {
	return (s.header & 0x80) != 0
}

func (s *CodecZenohOam) SetZ(v bool) {
	if v {
		s.header |= 0x80
	} else {
		s.header &^= 0x80
	}
}

// Encode writes the CodecZenohOam into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohOam) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohOam) EncodeToBytes() []byte {
	_dst := make([]byte, 0)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
