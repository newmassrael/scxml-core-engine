// SCE-MAP: codec_zenoh_undecl_subscriber:46

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_undecl_subscriber

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_decl_ext_keyexpr"
)

// CodecZenohUndeclSubscriber represents the codec frame layout.
type CodecZenohUndeclSubscriber struct {
	Id uint32
	ExtKeyexpr *codec_zenoh_decl_ext_keyexpr.CodecZenohDeclExtKeyexpr
}

// DecodeCodecZenohUndeclSubscriber decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohUndeclSubscriber(cursor *codec.SceCursor, Z byte) (*CodecZenohUndeclSubscriber, error) {
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
	Id, err := cursor.ReadVLEU32()
	if err != nil { return nil, err }
	var ExtKeyexpr *codec_zenoh_decl_ext_keyexpr.CodecZenohDeclExtKeyexpr
	if (Z & 0x01) != 0 {
		_emb, err := codec_zenoh_decl_ext_keyexpr.DecodeCodecZenohDeclExtKeyexpr(cursor)
		if err != nil {
			return nil, err
		}
		ExtKeyexpr = _emb
	}
	return &CodecZenohUndeclSubscriber{
		Id: Id,
		ExtKeyexpr: ExtKeyexpr,
	}, nil
}

// Encode writes the CodecZenohUndeclSubscriber into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohUndeclSubscriber) Encode(w codec.SceSink, Z byte) error {
	// Streaming cursor encode (SSOT selection: `needs_streaming`).
	// Mirrors the streaming decode: every field appends its own bytes in
	// declaration order through the per-field encode blocks, so a gated
	// field skips its append when absent, and a fixed field after a
	// variable-length payload lands after the payload (the positional path
	// appends variable fields last, placing it ahead on the wire).
	// Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
	// dedicated helpers; everything else uses `present_if_encode_block`.
	if err := codec.WriteVLEU32(w, uint32(s.Id)); err != nil {
		return err
	}
	if s.ExtKeyexpr != nil {
		if err := s.ExtKeyexpr.Encode(w); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohUndeclSubscriber) EncodeToBytes(Z byte) []byte {
	_dst := make([]byte, 0, 261)
	_ = s.Encode(codec.NewBytesSink(&_dst), Z)
	return _dst
}
