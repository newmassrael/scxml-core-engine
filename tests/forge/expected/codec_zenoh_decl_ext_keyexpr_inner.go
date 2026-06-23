// SCE-MAP: codec_zenoh_decl_ext_keyexpr_inner:64

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_decl_ext_keyexpr_inner

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohDeclExtKeyexprInner represents the codec frame layout.
type CodecZenohDeclExtKeyexprInner struct {
	InnerHeader uint8
	Id uint64
	Suffix []byte
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
	// (runtime presence; `*T` / nil `[]byte`), VLE / repeat / TLV-chain /
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
	var Suffix []byte
	if (InnerHeader & 0x01) != 0 {
		_n := cursor.Remaining()
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Suffix = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	return &CodecZenohDeclExtKeyexprInner{
		InnerHeader: InnerHeader,
		Id: Id,
		Suffix: Suffix,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohDeclExtKeyexprInner) N() bool {
	return (s.InnerHeader & 0x01) != 0
}

func (s *CodecZenohDeclExtKeyexprInner) SetN(v bool) {
	if v {
		s.InnerHeader |= 0x01
	} else {
		s.InnerHeader &^= 0x01
	}
}

func (s *CodecZenohDeclExtKeyexprInner) M() bool {
	return (s.InnerHeader & 0x02) != 0
}

func (s *CodecZenohDeclExtKeyexprInner) SetM(v bool) {
	if v {
		s.InnerHeader |= 0x02
	} else {
		s.InnerHeader &^= 0x02
	}
}

// Encode writes the CodecZenohDeclExtKeyexprInner into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohDeclExtKeyexprInner) Encode(w codec.SceSink) error {
	// Streaming cursor encode (SSOT selection: `needs_streaming`).
	// Mirrors the streaming decode: every field appends its own bytes in
	// declaration order through the per-field encode blocks, so a gated
	// field skips its append when absent, and a fixed field after a
	// variable-length payload lands after the payload (the positional path
	// appends variable fields last, placing it ahead on the wire).
	// Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
	// dedicated helpers; everything else uses `present_if_encode_block`.
	if err := w.WriteBytes([]byte{ s.InnerHeader }); err != nil {
		return err
	}
	{
		_vle := uint64(s.Id)
		_vn := 0
		for _vle >= 0x80 && _vn < 8 {
			if err := w.WriteBytes([]byte{ byte(_vle&0x7F) | 0x80 }); err != nil {
				return err
			}
			_vle >>= 7
			_vn++
		}
		if err := w.WriteBytes([]byte{ byte(_vle) }); err != nil {
			return err
		}
	}
	if s.Suffix != nil {
		if err := w.WriteBytes(s.Suffix); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohDeclExtKeyexprInner) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 138)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
