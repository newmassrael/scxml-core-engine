// SCE-MAP: codec_chain_prev_tail:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_chain_prev_tail

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_chain_prev_entry"
)

// CodecChainPrevTail represents the codec frame layout.
type CodecChainPrevTail struct {
	Head uint8
	Entries []codec_chain_prev_entry.CodecChainPrevEntry
}

// DecodeCodecChainPrevTail decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecChainPrevTail(cursor *codec.SceCursor) (*CodecChainPrevTail, error) {
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
	var Head uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Head = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	Entries := make([]codec_chain_prev_entry.CodecChainPrevEntry, 0, 4)
	var _prev_entries_after_marker byte
	for _i := 0; _i < int(4); _i++ {
		if cursor.Remaining() == 0 {
			break
		}
		_elem, err := codec_chain_prev_entry.DecodeCodecChainPrevEntry(cursor, _prev_entries_after_marker, byte((Head >> 0) & 0x1))
		if err != nil {
			return nil, err
		}
		_prev_entries_after_marker = 0
		if uint64(_elem.ExtId()) == 4 {
			_prev_entries_after_marker = 1
		}
		Entries = append(Entries, *_elem)
	}
	if cursor.Remaining() > 0 {
		return nil, codec.ErrTlvChainOverflow
	}
	return &CodecChainPrevTail{
		Head: Head,
		Entries: Entries,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecChainPrevTail) T() bool {
	return (s.Head & 0x01) != 0
}

func (s *CodecChainPrevTail) SetT(v bool) {
	if v {
		s.Head |= 0x01
	} else {
		s.Head &^= 0x01
	}
}

// Encode writes the CodecChainPrevTail into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecChainPrevTail) Encode(w codec.SceSink) error {
	// Streaming cursor encode (SSOT selection: `needs_streaming`).
	// Mirrors the streaming decode: every field appends its own bytes in
	// declaration order through the per-field encode blocks, so a gated
	// field skips its append when absent, and a fixed field after a
	// variable-length payload lands after the payload (the positional path
	// appends variable fields last, placing it ahead on the wire).
	// Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
	// dedicated helpers; everything else uses `present_if_encode_block`.
	if err := w.WriteBytes([]byte{ s.Head }); err != nil {
		return err
	}
	var _prev_entries_after_marker byte
	for _i := range s.Entries {
		if err := s.Entries[_i].Encode(w, _prev_entries_after_marker, byte((s.Head >> 0) & 0x1)); err != nil {
			return err
		}
		_prev_entries_after_marker = 0
		if uint64(s.Entries[_i].ExtId()) == 4 {
			_prev_entries_after_marker = 1
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecChainPrevTail) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 21)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
