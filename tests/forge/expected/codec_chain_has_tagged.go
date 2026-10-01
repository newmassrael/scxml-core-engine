// SCE-MAP: codec_chain_has_tagged:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_chain_has_tagged

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_chain_has_tagged_entry"
)

// CodecChainHasTagged represents the codec frame layout.
type CodecChainHasTagged struct {
	Header uint8
	Entries []codec_chain_has_tagged_entry.CodecChainHasTaggedEntry
	Priority *uint8
	Checksum *uint16
}

// DecodeCodecChainHasTagged decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecChainHasTagged(cursor *codec.SceCursor) (*CodecChainHasTagged, error) {
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
	Entries := make([]codec_chain_has_tagged_entry.CodecChainHasTaggedEntry, 0, 3)
	_more := false
	for _i := 0; _i < int(3); _i++ {
		if cursor.Remaining() == 0 {
			break
		}
		_elem, err := codec_chain_has_tagged_entry.DecodeCodecChainHasTaggedEntry(cursor)
		if err != nil {
			return nil, err
		}
		_more = _elem.More()
		Entries = append(Entries, *_elem)
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
	_has_Entries_7 := false
	for _, _e := range Entries {
		if uint64(_e.EntryType) == 7 {
			_has_Entries_7 = true
			break
		}
	}
	_has_Entries_9 := false
	for _, _e := range Entries {
		if uint64(_e.EntryType) == 9 {
			_has_Entries_9 = true
			break
		}
	}
	var Priority *uint8
	if _has_Entries_7 || (Header & 0x01) != 0 {
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		_v := raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
		Priority = &_v
	}
	var Checksum *uint16
	if !_has_Entries_9 {
		raw, err := cursor.PeekSlice(2)
		if err != nil {
			return nil, err
		}
		_v := uint16(raw[0])<<8 | uint16(raw[1])
		if err := cursor.Advance(2); err != nil {
			return nil, err
		}
		Checksum = &_v
	}
	return &CodecChainHasTagged{
		Header: Header,
		Entries: Entries,
		Priority: Priority,
		Checksum: Checksum,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecChainHasTagged) Wide() bool {
	return (s.Header & 0x01) != 0
}

func (s *CodecChainHasTagged) SetWide(v bool) {
	if v {
		s.Header |= 0x01
	} else {
		s.Header &^= 0x01
	}
}

// Encode writes the CodecChainHasTagged into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecChainHasTagged) Encode(w codec.SceSink) error {
	_has_Entries_7 := false
	for _, _e := range s.Entries {
		if uint64(_e.EntryType) == 7 {
			_has_Entries_7 = true
			break
		}
	}
	_has_Entries_9 := false
	for _, _e := range s.Entries {
		if uint64(_e.EntryType) == 9 {
			_has_Entries_9 = true
			break
		}
	}
	if _has_Entries_7 || (s.Header & 0x01) != 0 {
		if s.Priority == nil {
			return codec.ErrPresentIfMismatch
		}
	} else if s.Priority != nil {
		return codec.ErrPresentIfMismatch
	}
	if !_has_Entries_9 {
		if s.Checksum == nil {
			return codec.ErrPresentIfMismatch
		}
	} else if s.Checksum != nil {
		return codec.ErrPresentIfMismatch
	}
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
	for _i := range s.Entries {
		if err := s.Entries[_i].Encode(w); err != nil {
			return err
		}
	}
	if s.Priority != nil {
		_v := *s.Priority
		if err := w.WriteBytes([]byte{ _v }); err != nil {
			return err
		}
	}
	if s.Checksum != nil {
		_v := *s.Checksum
		if err := w.WriteBytes([]byte{ byte(_v >> 8 & 0xFF) }); err != nil {
			return err
		}
		if err := w.WriteBytes([]byte{ byte(_v & 0xFF) }); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice, or the
// error Encode refuses a message with when a field's presence disagrees
// with the chain-membership `sce:present-if` that gates it
// (codec.ErrPresentIfMismatch). Callers targeting zero-alloc hot paths
// should call Encode directly against a caller-owned sink (e.g.
// BoundedSink over a stack buffer).
func (s *CodecChainHasTagged) EncodeToBytes() ([]byte, error) {
	_dst := make([]byte, 0, 14)
	if err := s.Encode(codec.NewBytesSink(&_dst)); err != nil {
		return nil, err
	}
	return _dst, nil
}
