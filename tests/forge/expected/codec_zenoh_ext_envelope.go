// SCE-MAP: codec_zenoh_ext_envelope:35

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_ext_envelope

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_ext_entry"
)

// CodecZenohExtEnvelope represents the codec frame layout.
type CodecZenohExtEnvelope struct {
	HeaderFlags uint8
	Extensions []codec_zenoh_ext_entry.CodecZenohExtEntry
}

// DecodeCodecZenohExtEnvelope decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohExtEnvelope(cursor *codec.SceCursor) (*CodecZenohExtEnvelope, error) {
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
	var HeaderFlags uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		HeaderFlags = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	Extensions := make([]codec_zenoh_ext_entry.CodecZenohExtEntry, 0, 8)
	_more := false
	for _i := 0; _i < int(8); _i++ {
		if cursor.Remaining() == 0 {
			break
		}
		_elem, err := codec_zenoh_ext_entry.DecodeCodecZenohExtEntry(cursor)
		if err != nil {
			return nil, err
		}
		_more = _elem.Z()
		Extensions = append(Extensions, *_elem)
		if !_more {
			break
		}
	}
	if _more && cursor.Remaining() == 0 {
		return nil, codec.ErrNeedMoreBytes
	}
	if _more {
		return nil, codec.ErrTlvChainOverflow
	}
	return &CodecZenohExtEnvelope{
		HeaderFlags: HeaderFlags,
		Extensions: Extensions,
	}, nil
}

// Encode writes the CodecZenohExtEnvelope into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohExtEnvelope) Encode(w codec.SceSink) error {
	// Streaming cursor encode (SSOT selection: `needs_streaming`).
	// Mirrors the streaming decode: every field appends its own bytes in
	// declaration order through the per-field encode blocks, so a gated
	// field skips its append when absent, and a fixed field after a
	// variable-length payload lands after the payload (the positional path
	// appends variable fields last, placing it ahead on the wire).
	// Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
	// dedicated helpers; everything else uses `present_if_encode_block`.
	if err := w.WriteBytes([]byte{ s.HeaderFlags }); err != nil {
		return err
	}
	for _i := range s.Extensions {
		if err := s.Extensions[_i].Encode(w); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohExtEnvelope) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 337)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
