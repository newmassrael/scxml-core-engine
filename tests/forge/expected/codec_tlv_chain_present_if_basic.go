// SCE-MAP: codec_tlv_chain_present_if_basic:37 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_tlv_chain_present_if_basic

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_tlv_entry"
)

// CodecTlvChainPresentIfBasic represents the codec frame layout.
type CodecTlvChainPresentIfBasic struct {
	Carrier uint8
	Entries []codec_tlv_entry.CodecTlvEntry
}

// DecodeCodecTlvChainPresentIfBasic decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecTlvChainPresentIfBasic(cursor *codec.SceCursor) (*CodecTlvChainPresentIfBasic, error) {
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
	var Carrier uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Carrier = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var Entries []codec_tlv_entry.CodecTlvEntry
	if (Carrier & 0x01) != 0 {
		Entries = make([]codec_tlv_entry.CodecTlvEntry, 0, 4)
		for _i := 0; _i < int(4); _i++ {
			if cursor.Remaining() == 0 {
				break
			}
			_elem, err := codec_tlv_entry.DecodeCodecTlvEntry(cursor)
			if err != nil {
				return nil, err
			}
			Entries = append(Entries, *_elem)
		}
		if cursor.Remaining() > 0 {
			return nil, codec.ErrTlvChainOverflow
		}
	}
	return &CodecTlvChainPresentIfBasic{
		Carrier: Carrier,
		Entries: Entries,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecTlvChainPresentIfBasic) HasChain() bool {
	return (s.Carrier & 0x01) != 0
}

func (s *CodecTlvChainPresentIfBasic) SetHasChain(v bool) {
	if v {
		s.Carrier |= 0x01
	} else {
		s.Carrier &^= 0x01
	}
}

// Encode writes the CodecTlvChainPresentIfBasic into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecTlvChainPresentIfBasic) Encode(w codec.SceSink) error {
	// Streaming cursor encode (SSOT selection: `needs_streaming`).
	// Mirrors the streaming decode: every field appends its own bytes in
	// declaration order through the per-field encode blocks, so a gated
	// field skips its append when absent, and a fixed field after a
	// variable-length payload lands after the payload (the positional path
	// appends variable fields last, placing it ahead on the wire).
	// Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
	// dedicated helpers; everything else uses `present_if_encode_block`.
	if err := w.WriteBytes([]byte{ s.Carrier }); err != nil {
		return err
	}
	for _i := range s.Entries {
		if err := s.Entries[_i].Encode(w); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecTlvChainPresentIfBasic) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 137)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
