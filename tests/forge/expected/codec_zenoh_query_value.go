// SCE-MAP: codec_zenoh_query_value:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_query_value

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_value_entry"
)

// CodecZenohQueryValue represents the codec frame layout.
type CodecZenohQueryValue struct {
	Exts []codec_zenoh_value_entry.CodecZenohValueEntry
}

// DecodeCodecZenohQueryValue decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohQueryValue(cursor *codec.SceCursor) (*CodecZenohQueryValue, error) {
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
	Exts := make([]codec_zenoh_value_entry.CodecZenohValueEntry, 0, 4)
	var _prev_exts_after_shm byte
	_more := false
	for _i := 0; _i < int(4); _i++ {
		if cursor.Remaining() == 0 {
			break
		}
		_elem, err := codec_zenoh_value_entry.DecodeCodecZenohValueEntry(cursor, _prev_exts_after_shm)
		if err != nil {
			return nil, err
		}
		_more = _elem.Z()
		_prev_exts_after_shm = 0
		if uint64(_elem.ExtId()) == 4 {
			_prev_exts_after_shm = 1
		}
		Exts = append(Exts, *_elem)
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
	return &CodecZenohQueryValue{
		Exts: Exts,
	}, nil
}

// Encode writes the CodecZenohQueryValue into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohQueryValue) Encode(w codec.SceSink) error {
	// Streaming cursor encode (SSOT selection: `needs_streaming`).
	// Mirrors the streaming decode: every field appends its own bytes in
	// declaration order through the per-field encode blocks, so a gated
	// field skips its append when absent, and a fixed field after a
	// variable-length payload lands after the payload (the positional path
	// appends variable fields last, placing it ahead on the wire).
	// Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
	// dedicated helpers; everything else uses `present_if_encode_block`.
	var _prev_exts_after_shm byte
	for _i := range s.Exts {
		if err := s.Exts[_i].Encode(w, _prev_exts_after_shm); err != nil {
			return err
		}
		_prev_exts_after_shm = 0
		if uint64(s.Exts[_i].ExtId()) == 4 {
			_prev_exts_after_shm = 1
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohQueryValue) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 656)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
