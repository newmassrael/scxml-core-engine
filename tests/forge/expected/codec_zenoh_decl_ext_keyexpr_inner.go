// SCE-MAP: codec_zenoh_decl_ext_keyexpr_inner:65 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_decl_ext_keyexpr_inner

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
)

// CodecZenohDeclExtKeyexprInnerSuffixBound is the bound of the suffix element of CodecZenohDeclExtKeyexprInner: 0 to 65535 bytes
// (docs/adr/0077 §3). A Bytes of it is made only within it.
type CodecZenohDeclExtKeyexprInnerSuffixBound struct{}

// Least is the fewest bytes the element holds.
func (CodecZenohDeclExtKeyexprInnerSuffixBound) Least() int { return 0 }

// Most is the most bytes the element holds.
func (CodecZenohDeclExtKeyexprInnerSuffixBound) Most() int { return 65535 }

// CodecZenohDeclExtKeyexprInnerSource is what the encode of CodecZenohDeclExtKeyexprInner reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohDeclExtKeyexprInner is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohDeclExtKeyexprInnerSource interface {
	InnerHeader() uint8
	Id() uint64
	Suffix() optional.Of[codec.Bytes[CodecZenohDeclExtKeyexprInnerSuffixBound]]
}

// CodecZenohDeclExtKeyexprInner represents the codec frame layout.
type CodecZenohDeclExtKeyexprInner struct {
	innerHeader uint8
	id uint64
	suffix optional.Of[codec.Bytes[CodecZenohDeclExtKeyexprInnerSuffixBound]]
}

// MakeCodecZenohDeclExtKeyexprInner returns the CodecZenohDeclExtKeyexprInner that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohDeclExtKeyexprInner, or as its zero value, and is read through its methods.
func MakeCodecZenohDeclExtKeyexprInner(innerHeader uint8, id uint64, suffix optional.Of[codec.Bytes[CodecZenohDeclExtKeyexprInnerSuffixBound]]) CodecZenohDeclExtKeyexprInner {
	return CodecZenohDeclExtKeyexprInner{innerHeader: innerHeader, id: id, suffix: suffix}
}

// InnerHeader returns the element the value presents as InnerHeader.
func (s *CodecZenohDeclExtKeyexprInner) InnerHeader() uint8 {
	return s.innerHeader
}

// Id returns the element the value presents as Id.
func (s *CodecZenohDeclExtKeyexprInner) Id() uint64 {
	return s.id
}

// Suffix returns the element the value presents as Suffix.
func (s *CodecZenohDeclExtKeyexprInner) Suffix() optional.Of[codec.Bytes[CodecZenohDeclExtKeyexprInnerSuffixBound]] {
	return s.suffix
}

// CodecZenohDeclExtKeyexprInnerOverrides holds the one-bit flags of CodecZenohDeclExtKeyexprInner a parent may
// set in place of the ones a value presents, such as the flag a chain whose
// entries are CodecZenohDeclExtKeyexprInner ends them on, which the chain derives from where
// each entry stands. A flag it leaves absent is the one the value presents.
type CodecZenohDeclExtKeyexprInnerOverrides struct {
	// InnerHeaderM is the `M` flag of `inner_header`, where a parent sets it.
	InnerHeaderM optional.Of[bool]
}

// EncodeSource writes any source of CodecZenohDeclExtKeyexprInner into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohDeclExtKeyexprInnerSource, w codec.SceSink) error {
	return EncodeSourceOverriding(src, w, CodecZenohDeclExtKeyexprInnerOverrides{})
}

// EncodeSourceOverriding is EncodeSource, each flag overrides sets written in
// place of the one src presents.
func EncodeSourceOverriding(src CodecZenohDeclExtKeyexprInnerSource, w codec.SceSink, overrides CodecZenohDeclExtKeyexprInnerOverrides) error {
	_cInnerHeader := src.InnerHeader()
	_cInnerHeader &^= 0x1
	if src.Suffix().IsPresent() {
		_cInnerHeader |= 0x1
	}
	if _o, ok := overrides.InnerHeaderM.Get(); ok {
		_cInnerHeader &^= 0x2
		if _o {
			_cInnerHeader |= 0x2
		}
	}
	if err := w.WriteBytes([]byte{byte(_cInnerHeader)}); err != nil {
		return err
	}
	if err := codec.WriteVLEU64(w, uint64(src.Id())); err != nil {
		return err
	}
	if src.Suffix().IsPresent() {
		if err := w.WriteBytes(src.Suffix().Proven().Bytes()); err != nil {
			return err
		}
	}
	return nil
}

// DecodeCodecZenohDeclExtKeyexprInner decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohDeclExtKeyexprInner(cursor *codec.SceCursor) (*CodecZenohDeclExtKeyexprInner, error) {
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
	var InnerHeader uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		InnerHeader = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	Id, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var Suffix optional.Of[[]byte]
	if (InnerHeader & 0x01) != 0 {
		_n := cursor.Remaining()
		if _n > 65535 {
			return nil, codec.ErrLengthOutOfRange
		}
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Suffix = optional.Some(append([]byte(nil), raw...))
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	_bSuffix, err := codec.MakeOptionalBytes[CodecZenohDeclExtKeyexprInnerSuffixBound](Suffix)
	if err != nil {
		return nil, err
	}
	return &CodecZenohDeclExtKeyexprInner{
		innerHeader: InnerHeader,
		id: Id,
		suffix: _bSuffix,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohDeclExtKeyexprInner) N() bool {
	return (s.innerHeader & 0x01) != 0
}

func (s *CodecZenohDeclExtKeyexprInner) SetN(v bool) {
	if v {
		s.innerHeader |= 0x01
	} else {
		s.innerHeader &^= 0x01
	}
}

func (s *CodecZenohDeclExtKeyexprInner) M() bool {
	return (s.innerHeader & 0x02) != 0
}

func (s *CodecZenohDeclExtKeyexprInner) SetM(v bool) {
	if v {
		s.innerHeader |= 0x02
	} else {
		s.innerHeader &^= 0x02
	}
}

// Encode writes the CodecZenohDeclExtKeyexprInner into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohDeclExtKeyexprInner) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohDeclExtKeyexprInner) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 65545)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
