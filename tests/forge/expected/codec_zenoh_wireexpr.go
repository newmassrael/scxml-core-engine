// SCE-MAP: codec_zenoh_wireexpr:53 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_wireexpr

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"unicode/utf8"
)

// CodecZenohWireexpr represents the codec frame layout.
type CodecZenohWireexpr struct {
	Id uint64
	SuffixLen *uint64
	Suffix *string
}

// DecodeCodecZenohWireexpr decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohWireexpr(cursor *codec.SceCursor, N byte) (*CodecZenohWireexpr, error) {
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
	Id, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var SuffixLen *uint64
	if (N & 0x01) != 0 {
		_v, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
		SuffixLen = &_v
	}
	var Suffix *string
	if (N & 0x01) != 0 {
		_n := int(*SuffixLen)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		if !utf8.Valid(raw) {
			return nil, codec.ErrInvalidUTF8
		}
		_v := string(raw)
		Suffix = &_v
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	return &CodecZenohWireexpr{
		Id: Id,
		SuffixLen: SuffixLen,
		Suffix: Suffix,
	}, nil
}

// Encode writes the CodecZenohWireexpr into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohWireexpr) Encode(w codec.SceSink, N byte) error {
	// Streaming cursor encode (SSOT selection: `needs_streaming`).
	// Mirrors the streaming decode: every field appends its own bytes in
	// declaration order through the per-field encode blocks, so a gated
	// field skips its append when absent, and a fixed field after a
	// variable-length payload lands after the payload (the positional path
	// appends variable fields last, placing it ahead on the wire).
	// Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
	// dedicated helpers; everything else uses `present_if_encode_block`.
	if err := codec.WriteVLEU64(w, uint64(s.Id)); err != nil {
		return err
	}
	if s.SuffixLen != nil {
		_v := *s.SuffixLen
	if err := codec.WriteVLEU64(w, uint64(_v)); err != nil {
		return err
	}
	}
	if s.Suffix != nil {
		if err := w.WriteBytes([]byte(*s.Suffix)); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohWireexpr) EncodeToBytes(N byte) []byte {
	_dst := make([]byte, 0, 146)
	_ = s.Encode(codec.NewBytesSink(&_dst), N)
	return _dst
}
