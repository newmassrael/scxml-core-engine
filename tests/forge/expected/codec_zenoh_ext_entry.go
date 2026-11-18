// SCE-MAP: codec_zenoh_ext_entry:52 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_ext_entry

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
	"example.com/sce-forge/codec_zenoh_ext_unit"
	"example.com/sce-forge/codec_zenoh_ext_zint"
	"example.com/sce-forge/codec_zenoh_ext_zbuf"
)

// CodecZenohExtEntryUnknownTag is the tag the catch-all arm of CodecZenohExtEntry's variant holds: one
// the bits the tag is read from hold and no arm names, held to that where a
// value of it is made (docs/adr/0065 §6 R3, docs/adr/0081 §2), so the catch-all
// a value holds is written and read back as the catch-all. Its zero value
// holds 0x3, the least such tag.
type CodecZenohExtEntryUnknownTag struct {
	tag uint8
	set bool
}

// MakeCodecZenohExtEntryUnknownTag returns tag as the tag of the catch-all, and false where an
// arm names it or the bits the tag is read from do not hold it.
func MakeCodecZenohExtEntryUnknownTag(tag uint8) (CodecZenohExtEntryUnknownTag, bool) {
	if tag <= 0x2 || tag > 0x3 {
		return CodecZenohExtEntryUnknownTag{}, false
	}
	return CodecZenohExtEntryUnknownTag{tag: tag, set: true}, true
}

// Value returns the tag.
func (t CodecZenohExtEntryUnknownTag) Value() uint8 {
	if !t.set {
		return 0x3
	}
	return t.tag
}

// CodecZenohExtEntryVariant is the arm of the variant a CodecZenohExtEntry holds (docs/adr/0081 §2):
// one of CodecZenohExtEntryVariantCodecZenohExtUnit, CodecZenohExtEntryVariantCodecZenohExtZint, CodecZenohExtEntryVariantCodecZenohExtZbuf, CodecZenohExtEntryVariantDefault, which only this package declares.
type CodecZenohExtEntryVariant interface {
	isCodecZenohExtEntryVariant()
}

// CodecZenohExtEntryVariantCodecZenohExtUnit is the arm of CodecZenohExtEntryVariant whose body is a CodecZenohExtUnit, chosen by the tag 0x0.
type CodecZenohExtEntryVariantCodecZenohExtUnit struct {
	body codec_zenoh_ext_unit.CodecZenohExtUnitSource
}

// MakeCodecZenohExtEntryVariantCodecZenohExtUnit returns the arm that holds what it is handed.
func MakeCodecZenohExtEntryVariantCodecZenohExtUnit(body codec_zenoh_ext_unit.CodecZenohExtUnitSource) CodecZenohExtEntryVariantCodecZenohExtUnit {
	return CodecZenohExtEntryVariantCodecZenohExtUnit{body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecZenohExtUnit its codec makes by default.
func (a CodecZenohExtEntryVariantCodecZenohExtUnit) Body() codec_zenoh_ext_unit.CodecZenohExtUnitSource {
	if a.body == nil {
		return &codec_zenoh_ext_unit.CodecZenohExtUnit{}
	}
	return a.body
}

func (CodecZenohExtEntryVariantCodecZenohExtUnit) isCodecZenohExtEntryVariant() {}

// CodecZenohExtEntryVariantCodecZenohExtZint is the arm of CodecZenohExtEntryVariant whose body is a CodecZenohExtZint, chosen by the tag 0x1.
type CodecZenohExtEntryVariantCodecZenohExtZint struct {
	body codec_zenoh_ext_zint.CodecZenohExtZintSource
}

// MakeCodecZenohExtEntryVariantCodecZenohExtZint returns the arm that holds what it is handed.
func MakeCodecZenohExtEntryVariantCodecZenohExtZint(body codec_zenoh_ext_zint.CodecZenohExtZintSource) CodecZenohExtEntryVariantCodecZenohExtZint {
	return CodecZenohExtEntryVariantCodecZenohExtZint{body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecZenohExtZint its codec makes by default.
func (a CodecZenohExtEntryVariantCodecZenohExtZint) Body() codec_zenoh_ext_zint.CodecZenohExtZintSource {
	if a.body == nil {
		return &codec_zenoh_ext_zint.CodecZenohExtZint{}
	}
	return a.body
}

func (CodecZenohExtEntryVariantCodecZenohExtZint) isCodecZenohExtEntryVariant() {}

// CodecZenohExtEntryVariantCodecZenohExtZbuf is the arm of CodecZenohExtEntryVariant whose body is a CodecZenohExtZbuf, chosen by the tag 0x2.
type CodecZenohExtEntryVariantCodecZenohExtZbuf struct {
	body codec_zenoh_ext_zbuf.CodecZenohExtZbufSource
}

// MakeCodecZenohExtEntryVariantCodecZenohExtZbuf returns the arm that holds what it is handed.
func MakeCodecZenohExtEntryVariantCodecZenohExtZbuf(body codec_zenoh_ext_zbuf.CodecZenohExtZbufSource) CodecZenohExtEntryVariantCodecZenohExtZbuf {
	return CodecZenohExtEntryVariantCodecZenohExtZbuf{body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecZenohExtZbuf its codec makes by default.
func (a CodecZenohExtEntryVariantCodecZenohExtZbuf) Body() codec_zenoh_ext_zbuf.CodecZenohExtZbufSource {
	if a.body == nil {
		return &codec_zenoh_ext_zbuf.CodecZenohExtZbuf{}
	}
	return a.body
}

func (CodecZenohExtEntryVariantCodecZenohExtZbuf) isCodecZenohExtEntryVariant() {}

// CodecZenohExtEntryVariantDefault is the arm of CodecZenohExtEntryVariant whose body is a CodecZenohExtUnit, chosen by every tag no arm names.
type CodecZenohExtEntryVariantDefault struct {
	tag  CodecZenohExtEntryUnknownTag
	body codec_zenoh_ext_unit.CodecZenohExtUnitSource
}

// MakeCodecZenohExtEntryVariantDefault returns the arm that holds what it is handed.
func MakeCodecZenohExtEntryVariantDefault(tag CodecZenohExtEntryUnknownTag, body codec_zenoh_ext_unit.CodecZenohExtUnitSource) CodecZenohExtEntryVariantDefault {
	return CodecZenohExtEntryVariantDefault{tag: tag, body: body}
}

// Body returns the body the arm holds; an arm that holds none, as the zero
// arm, holds the CodecZenohExtUnit its codec makes by default.
func (a CodecZenohExtEntryVariantDefault) Body() codec_zenoh_ext_unit.CodecZenohExtUnitSource {
	if a.body == nil {
		return &codec_zenoh_ext_unit.CodecZenohExtUnit{}
	}
	return a.body
}

// Tag returns the tag the arm holds, which no arm names.
func (a CodecZenohExtEntryVariantDefault) Tag() CodecZenohExtEntryUnknownTag {
	return a.tag
}

func (CodecZenohExtEntryVariantDefault) isCodecZenohExtEntryVariant() {}

// CodecZenohExtEntrySource is what the encode of CodecZenohExtEntry reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohExtEntry is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohExtEntrySource interface {
	Header() uint8
	Body() CodecZenohExtEntryVariant
}

// CodecZenohExtEntry represents the codec frame layout.
type CodecZenohExtEntry struct {
	header uint8
	body CodecZenohExtEntryVariant
}

// MakeCodecZenohExtEntry returns the CodecZenohExtEntry that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohExtEntry, or as its zero value, and is read through its methods.
func MakeCodecZenohExtEntry(header uint8, body CodecZenohExtEntryVariant) CodecZenohExtEntry {
	return CodecZenohExtEntry{header: header, body: body}
}

// Header returns the element the value presents as Header.
func (s *CodecZenohExtEntry) Header() uint8 {
	return s.header
}

// Body returns the arm of the variant the value holds; a value that
// was handed none, as the zero value, holds CodecZenohExtEntryVariantCodecZenohExtUnit{}.
func (s *CodecZenohExtEntry) Body() CodecZenohExtEntryVariant {
	if s.body == nil {
		return CodecZenohExtEntryVariantCodecZenohExtUnit{}
	}
	return s.body
}

// CodecZenohExtEntryOverrides holds the one-bit flags of CodecZenohExtEntry a parent may
// set in place of the ones a value presents, such as the flag a chain whose
// entries are CodecZenohExtEntry ends them on, which the chain derives from where
// each entry stands. A flag it leaves absent is the one the value presents.
type CodecZenohExtEntryOverrides struct {
	// HeaderM is the `M` flag of `header`, where a parent sets it.
	HeaderM optional.Of[bool]
	// HeaderZ is the `Z` flag of `header`, where a parent sets it.
	HeaderZ optional.Of[bool]
}

// EncodeSource writes any source of CodecZenohExtEntry into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohExtEntrySource, w codec.SceSink) error {
	return EncodeSourceOverriding(src, w, CodecZenohExtEntryOverrides{})
}

// EncodeSourceOverriding is EncodeSource, each flag overrides sets written in
// place of the one src presents.
func EncodeSourceOverriding(src CodecZenohExtEntrySource, w codec.SceSink, overrides CodecZenohExtEntryOverrides) error {
	_rBody := src.Body()
	var _tBody uint64
	switch _b := _rBody.(type) {
	case CodecZenohExtEntryVariantCodecZenohExtUnit:
		_tBody = 0x0
	case CodecZenohExtEntryVariantCodecZenohExtZint:
		_tBody = 0x1
	case CodecZenohExtEntryVariantCodecZenohExtZbuf:
		_tBody = 0x2
	case CodecZenohExtEntryVariantDefault:
		_tBody = uint64(_b.Tag().Value())
	}
	_cHeader := src.Header()
	_cHeader = (_cHeader &^ 0x60) | (uint8(_tBody & 0x3) << 5)
	if _o, ok := overrides.HeaderM.Get(); ok {
		_cHeader &^= 0x10
		if _o {
			_cHeader |= 0x10
		}
	}
	if _o, ok := overrides.HeaderZ.Get(); ok {
		_cHeader &^= 0x80
		if _o {
			_cHeader |= 0x80
		}
	}
	if err := w.WriteBytes([]byte{byte(_cHeader)}); err != nil {
		return err
	}
	switch _b := _rBody.(type) {
	case CodecZenohExtEntryVariantCodecZenohExtUnit:
		if err := codec_zenoh_ext_unit.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	case CodecZenohExtEntryVariantCodecZenohExtZint:
		if err := codec_zenoh_ext_zint.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	case CodecZenohExtEntryVariantCodecZenohExtZbuf:
		if err := codec_zenoh_ext_zbuf.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	case CodecZenohExtEntryVariantDefault:
		if err := codec_zenoh_ext_unit.EncodeSource(_b.Body(), w); err != nil {
			return err
		}
	default:
		panic("CodecZenohExtEntry: the source presents no arm of its variant")
	}
	return nil
}

// NewCodecZenohExtEntry returns a CodecZenohExtEntry initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecZenohExtEntry().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecZenohExtEntry{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecZenohExtEntry() *CodecZenohExtEntry {
	return &CodecZenohExtEntry{
	}
}

// DecodeCodecZenohExtEntry decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohExtEntry(cursor *codec.SceCursor) (*CodecZenohExtEntry, error) {
	// Decode fixed prefix (RFC §synth-5-B variant: fields before tag suffix).
	raw, err := cursor.PeekSlice(1)
	if err != nil {
		return nil, err
	}
	Header := raw[0]
	if err := cursor.Advance(1); err != nil {
		return nil, err
	}
	// Dispatch on the tag field; each arm decodes its body codec from
	// the cursor. The default arm (when declared) carries the runtime
	// tag value so encode can round-trip it back onto the wire.
	var body CodecZenohExtEntryVariant
	switch uint8((Header >> 5) & 0x03) {
	case 0:
		_arm, err := codec_zenoh_ext_unit.DecodeCodecZenohExtUnit(cursor)
		if err != nil {
			return nil, err
		}
		body = MakeCodecZenohExtEntryVariantCodecZenohExtUnit(_arm)
	case 1:
		_arm, err := codec_zenoh_ext_zint.DecodeCodecZenohExtZint(cursor)
		if err != nil {
			return nil, err
		}
		body = MakeCodecZenohExtEntryVariantCodecZenohExtZint(_arm)
	case 2:
		_arm, err := codec_zenoh_ext_zbuf.DecodeCodecZenohExtZbuf(cursor)
		if err != nil {
			return nil, err
		}
		body = MakeCodecZenohExtEntryVariantCodecZenohExtZbuf(_arm)
	default:
		_arm, err := codec_zenoh_ext_unit.DecodeCodecZenohExtUnit(cursor)
		if err != nil {
			return nil, err
		}
		_tag := uint8((Header >> 5) & 0x03)
		body = MakeCodecZenohExtEntryVariantDefault(CodecZenohExtEntryUnknownTag{tag: _tag, set: true}, _arm)
	}
	return &CodecZenohExtEntry{
		header: Header,
		body: body,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohExtEntry) ExtId() uint8 {
	return uint8((s.header >> 0) & 0x0F)
}

func (s *CodecZenohExtEntry) SetExtId(v uint8) {
	const _shiftedMask uint8 = 0x0F << 0
	_val := (uint8(v) & 0x0F) << 0
	s.header = (s.header &^ _shiftedMask) | _val
}

func (s *CodecZenohExtEntry) M() bool {
	return (s.header & 0x10) != 0
}

func (s *CodecZenohExtEntry) SetM(v bool) {
	if v {
		s.header |= 0x10
	} else {
		s.header &^= 0x10
	}
}

func (s *CodecZenohExtEntry) Enc() uint8 {
	return uint8((s.header >> 5) & 0x03)
}

func (s *CodecZenohExtEntry) SetEnc(v uint8) {
	const _shiftedMask uint8 = 0x03 << 5
	_val := (uint8(v) & 0x03) << 5
	s.header = (s.header &^ _shiftedMask) | _val
}

func (s *CodecZenohExtEntry) Z() bool {
	return (s.header & 0x80) != 0
}

func (s *CodecZenohExtEntry) SetZ(v bool) {
	if v {
		s.header |= 0x80
	} else {
		s.header &^= 0x80
	}
}

// Encode writes the CodecZenohExtEntry into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohExtEntry) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohExtEntry) EncodeToBytes() []byte {
	_dst := make([]byte, 0)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
