// SCE-MAP: codec_zenoh_interest_body:56 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_interest_body

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_wireexpr"
)

// CodecZenohInterestBody represents the codec frame layout.
type CodecZenohInterestBody struct {
	Header uint8
	Keyexpr *codec_zenoh_wireexpr.CodecZenohWireexpr
}

// DecodeCodecZenohInterestBody decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohInterestBody(cursor *codec.SceCursor) (*CodecZenohInterestBody, error) {
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
	var Keyexpr *codec_zenoh_wireexpr.CodecZenohWireexpr
	if (Header & 0x10) != 0 {
		_emb, err := codec_zenoh_wireexpr.DecodeCodecZenohWireexpr(cursor, byte((Header >> 5) & 0x1))
		if err != nil {
			return nil, err
		}
		Keyexpr = _emb
	}
	return &CodecZenohInterestBody{
		Header: Header,
		Keyexpr: Keyexpr,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohInterestBody) Keyexprs() bool {
	return (s.Header & 0x01) != 0
}

func (s *CodecZenohInterestBody) SetKeyexprs(v bool) {
	if v {
		s.Header |= 0x01
	} else {
		s.Header &^= 0x01
	}
}

func (s *CodecZenohInterestBody) Subscribers() bool {
	return (s.Header & 0x02) != 0
}

func (s *CodecZenohInterestBody) SetSubscribers(v bool) {
	if v {
		s.Header |= 0x02
	} else {
		s.Header &^= 0x02
	}
}

func (s *CodecZenohInterestBody) Queryables() bool {
	return (s.Header & 0x04) != 0
}

func (s *CodecZenohInterestBody) SetQueryables(v bool) {
	if v {
		s.Header |= 0x04
	} else {
		s.Header &^= 0x04
	}
}

func (s *CodecZenohInterestBody) Tokens() bool {
	return (s.Header & 0x08) != 0
}

func (s *CodecZenohInterestBody) SetTokens(v bool) {
	if v {
		s.Header |= 0x08
	} else {
		s.Header &^= 0x08
	}
}

func (s *CodecZenohInterestBody) Restricted() bool {
	return (s.Header & 0x10) != 0
}

func (s *CodecZenohInterestBody) SetRestricted(v bool) {
	if v {
		s.Header |= 0x10
	} else {
		s.Header &^= 0x10
	}
}

func (s *CodecZenohInterestBody) N() bool {
	return (s.Header & 0x20) != 0
}

func (s *CodecZenohInterestBody) SetN(v bool) {
	if v {
		s.Header |= 0x20
	} else {
		s.Header &^= 0x20
	}
}

func (s *CodecZenohInterestBody) M() bool {
	return (s.Header & 0x40) != 0
}

func (s *CodecZenohInterestBody) SetM(v bool) {
	if v {
		s.Header |= 0x40
	} else {
		s.Header &^= 0x40
	}
}

func (s *CodecZenohInterestBody) Aggregate() bool {
	return (s.Header & 0x80) != 0
}

func (s *CodecZenohInterestBody) SetAggregate(v bool) {
	if v {
		s.Header |= 0x80
	} else {
		s.Header &^= 0x80
	}
}

// Encode writes the CodecZenohInterestBody into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohInterestBody) Encode(w codec.SceSink) error {
	// Streaming cursor encode (SSOT selection: `needs_streaming`).
	// Mirrors the streaming decode: every field appends its own bytes in
	// declaration order through the per-field encode blocks, so a gated
	// field skips its append when absent, and a fixed field after a
	// variable-length payload lands after the payload (the positional path
	// appends variable fields last, placing it ahead on the wire).
	// Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
	// dedicated helpers; everything else uses `present_if_encode_block`.
	if err := w.WriteBytes([]byte{ s.Header }); err != nil {
		return err
	}
	if s.Keyexpr != nil {
		if err := s.Keyexpr.Encode(w, byte((s.Header >> 5) & 0x1)); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohInterestBody) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 257)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
