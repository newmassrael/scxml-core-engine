// SCE-MAP: codec_zenoh_decl_ext_keyexpr:89 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_decl_ext_keyexpr

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
	"example.com/sce-forge/codec_zenoh_decl_ext_keyexpr_inner"
)

// CodecZenohDeclExtKeyexprSource is what the encode of CodecZenohDeclExtKeyexpr reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohDeclExtKeyexpr is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohDeclExtKeyexprSource interface {
	OuterHeader() uint8
	Inner() codec_zenoh_decl_ext_keyexpr_inner.CodecZenohDeclExtKeyexprInnerSource
}

// CodecZenohDeclExtKeyexpr represents the codec frame layout.
type CodecZenohDeclExtKeyexpr struct {
	outerHeader uint8
	inner codec_zenoh_decl_ext_keyexpr_inner.CodecZenohDeclExtKeyexprInnerSource
}

// MakeCodecZenohDeclExtKeyexpr returns the CodecZenohDeclExtKeyexpr that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohDeclExtKeyexpr, or as its zero value, and is read through its methods.
func MakeCodecZenohDeclExtKeyexpr(outerHeader uint8, inner codec_zenoh_decl_ext_keyexpr_inner.CodecZenohDeclExtKeyexprInnerSource) CodecZenohDeclExtKeyexpr {
	return CodecZenohDeclExtKeyexpr{outerHeader: outerHeader, inner: inner}
}

// OuterHeader returns the element the value presents as OuterHeader.
func (s *CodecZenohDeclExtKeyexpr) OuterHeader() uint8 {
	return s.outerHeader
}

// Inner returns the element the value presents as Inner.
func (s *CodecZenohDeclExtKeyexpr) Inner() codec_zenoh_decl_ext_keyexpr_inner.CodecZenohDeclExtKeyexprInnerSource {
	if s.inner == nil {
		return &codec_zenoh_decl_ext_keyexpr_inner.CodecZenohDeclExtKeyexprInner{}
	}
	return s.inner
}

// WireTotalLength returns the total_length the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecZenohDeclExtKeyexpr) WireTotalLength() uint64 {
	var _k codec.CountingSink
	_ = codec_zenoh_decl_ext_keyexpr_inner.EncodeSource(s.Inner(), &_k)
	return uint64(_k.Position())
}

// CodecZenohDeclExtKeyexprOverrides holds the one-bit flags of CodecZenohDeclExtKeyexpr a parent may
// set in place of the ones a value presents, such as the flag a chain whose
// entries are CodecZenohDeclExtKeyexpr ends them on, which the chain derives from where
// each entry stands. A flag it leaves absent is the one the value presents.
type CodecZenohDeclExtKeyexprOverrides struct {
	// OuterHeaderM is the `M` flag of `outer_header`, where a parent sets it.
	OuterHeaderM optional.Of[bool]
	// OuterHeaderZ is the `Z` flag of `outer_header`, where a parent sets it.
	OuterHeaderZ optional.Of[bool]
}

// EncodeSource writes any source of CodecZenohDeclExtKeyexpr into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives total_length from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohDeclExtKeyexprSource, w codec.SceSink) error {
	return EncodeSourceOverriding(src, w, CodecZenohDeclExtKeyexprOverrides{})
}

// EncodeSourceOverriding is EncodeSource, each flag overrides sets written in
// place of the one src presents.
func EncodeSourceOverriding(src CodecZenohDeclExtKeyexprSource, w codec.SceSink, overrides CodecZenohDeclExtKeyexprOverrides) error {
	_cOuterHeader := src.OuterHeader()
	if _o, ok := overrides.OuterHeaderM.Get(); ok {
		_cOuterHeader &^= 0x10
		if _o {
			_cOuterHeader |= 0x10
		}
	}
	if _o, ok := overrides.OuterHeaderZ.Get(); ok {
		_cOuterHeader &^= 0x80
		if _o {
			_cOuterHeader |= 0x80
		}
	}
	var _kInner codec.CountingSink
	if err := codec_zenoh_decl_ext_keyexpr_inner.EncodeSource(src.Inner(), &_kInner); err != nil {
		return err
	}
	_dTotalLength := uint64(_kInner.Position())
	if err := w.WriteBytes([]byte{byte(_cOuterHeader)}); err != nil {
		return err
	}
	if err := codec.WriteVLEU64(w, uint64(_dTotalLength)); err != nil {
		return err
	}
	if err := codec_zenoh_decl_ext_keyexpr_inner.EncodeSource(src.Inner(), w); err != nil {
		return err
	}
	return nil
}

// DecodeCodecZenohDeclExtKeyexpr decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohDeclExtKeyexpr(cursor *codec.SceCursor) (*CodecZenohDeclExtKeyexpr, error) {
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
	var OuterHeader uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		OuterHeader = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	TotalLength, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var Inner codec_zenoh_decl_ext_keyexpr_inner.CodecZenohDeclExtKeyexprInner
	{
		_len := int(TotalLength)
		_raw, err := cursor.PeekSlice(_len)
		if err != nil {
			return nil, err
		}
		_inner := codec.NewSceCursor(_raw)
		_emb, err := codec_zenoh_decl_ext_keyexpr_inner.DecodeCodecZenohDeclExtKeyexprInner(&_inner)
		if err != nil {
			return nil, err
		}
		if err := cursor.Advance(_len); err != nil {
			return nil, err
		}
		Inner = *_emb
	}
	return &CodecZenohDeclExtKeyexpr{
		outerHeader: OuterHeader,
		inner: &Inner,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohDeclExtKeyexpr) ExtId() uint8 {
	return uint8((s.outerHeader >> 0) & 0x0F)
}

func (s *CodecZenohDeclExtKeyexpr) SetExtId(v uint8) {
	const _shiftedMask uint8 = 0x0F << 0
	_val := (uint8(v) & 0x0F) << 0
	s.outerHeader = (s.outerHeader &^ _shiftedMask) | _val
}

func (s *CodecZenohDeclExtKeyexpr) M() bool {
	return (s.outerHeader & 0x10) != 0
}

func (s *CodecZenohDeclExtKeyexpr) SetM(v bool) {
	if v {
		s.outerHeader |= 0x10
	} else {
		s.outerHeader &^= 0x10
	}
}

func (s *CodecZenohDeclExtKeyexpr) Enc() uint8 {
	return uint8((s.outerHeader >> 5) & 0x03)
}

func (s *CodecZenohDeclExtKeyexpr) SetEnc(v uint8) {
	const _shiftedMask uint8 = 0x03 << 5
	_val := (uint8(v) & 0x03) << 5
	s.outerHeader = (s.outerHeader &^ _shiftedMask) | _val
}

func (s *CodecZenohDeclExtKeyexpr) Z() bool {
	return (s.outerHeader & 0x80) != 0
}

func (s *CodecZenohDeclExtKeyexpr) SetZ(v bool) {
	if v {
		s.outerHeader |= 0x80
	} else {
		s.outerHeader &^= 0x80
	}
}

// Encode writes the CodecZenohDeclExtKeyexpr into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohDeclExtKeyexpr) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohDeclExtKeyexpr) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 65555)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
