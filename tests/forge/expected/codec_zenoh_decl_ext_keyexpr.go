// SCE-MAP: codec_zenoh_decl_ext_keyexpr:89

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_decl_ext_keyexpr

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_decl_ext_keyexpr_inner"
)

// CodecZenohDeclExtKeyexpr represents the codec frame layout.
type CodecZenohDeclExtKeyexpr struct {
	OuterHeader uint8
	TotalLength uint64
	Inner codec_zenoh_decl_ext_keyexpr_inner.CodecZenohDeclExtKeyexprInner
}

// DecodeCodecZenohDeclExtKeyexpr decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohDeclExtKeyexpr(cursor *codec.SceCursor) (*CodecZenohDeclExtKeyexpr, error) {
	// Streaming codec: each field reads from cursor directly
	// (VLE base-128 chain). Local var name reuses the Go-PascalCase
	// `field.id` — the struct literal's `Foo: Foo` is unambiguous
	// because the package owns both names. RFC §synth-5-B B4: per-field
	// bit-size dispatch routes Fixed / LengthRef siblings of VLE
	// fields through `present_if_decode_stmt` (predicate=None arms).
	// Pure-VLE codecs stay byte-stable.
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
		OuterHeader: OuterHeader,
		TotalLength: TotalLength,
		Inner: Inner,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohDeclExtKeyexpr) ExtId() uint8 {
	return uint8((s.OuterHeader >> 0) & 0x0F)
}

func (s *CodecZenohDeclExtKeyexpr) SetExtId(v uint8) {
	const _shiftedMask uint8 = 0x0F << 0
	_val := (uint8(v) & 0x0F) << 0
	s.OuterHeader = (s.OuterHeader &^ _shiftedMask) | _val
}

func (s *CodecZenohDeclExtKeyexpr) M() bool {
	return (s.OuterHeader & 0x10) != 0
}

func (s *CodecZenohDeclExtKeyexpr) SetM(v bool) {
	if v {
		s.OuterHeader |= 0x10
	} else {
		s.OuterHeader &^= 0x10
	}
}

func (s *CodecZenohDeclExtKeyexpr) Enc() uint8 {
	return uint8((s.OuterHeader >> 5) & 0x03)
}

func (s *CodecZenohDeclExtKeyexpr) SetEnc(v uint8) {
	const _shiftedMask uint8 = 0x03 << 5
	_val := (uint8(v) & 0x03) << 5
	s.OuterHeader = (s.OuterHeader &^ _shiftedMask) | _val
}

func (s *CodecZenohDeclExtKeyexpr) Z() bool {
	return (s.OuterHeader & 0x80) != 0
}

func (s *CodecZenohDeclExtKeyexpr) SetZ(v bool) {
	if v {
		s.OuterHeader |= 0x80
	} else {
		s.OuterHeader &^= 0x80
	}
}

// Encode writes the CodecZenohDeclExtKeyexpr into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohDeclExtKeyexpr) Encode(w codec.SceSink) error {
	// RFC §synth-5-B B4: per-field bit-size dispatch.
	if err := w.WriteBytes([]byte{ s.OuterHeader }); err != nil {
		return err
	}
	{
		_vle := uint64(s.TotalLength)
		for _vle >= 0x80 {
			if err := w.WriteBytes([]byte{ byte(_vle&0x7F) | 0x80 }); err != nil {
				return err
			}
			_vle >>= 7
		}
		if err := w.WriteBytes([]byte{ byte(_vle) }); err != nil {
			return err
		}
	}
	if err := s.Inner.Encode(w); err != nil {
		return err
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohDeclExtKeyexpr) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 267)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
