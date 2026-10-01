// SCE-MAP: codec_origin_envelope:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_origin_envelope

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"unicode/utf8"
	"example.com/sce-forge/codec_origin_leaf"
)

// CodecOriginEnvelope represents the codec frame layout.
type CodecOriginEnvelope struct {
	Hdr uint8
	NoteLen uint8
	Note string
	M uint8
	Required codec_origin_leaf.CodecOriginLeaf
	Optional *codec_origin_leaf.CodecOriginLeaf
	Items []codec_origin_leaf.CodecOriginLeaf
}

// DecodeCodecOriginEnvelope decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecOriginEnvelope(cursor *codec.SceCursor) (*CodecOriginEnvelope, error) {
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
	var Hdr uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Hdr = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var NoteLen uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		NoteLen = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var Note string
	{
		_n := int(NoteLen)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		if !utf8.Valid(raw) {
			return nil, codec.ErrInvalidUTF8
		}
		Note = string(raw)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	var M uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		M = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var Required codec_origin_leaf.CodecOriginLeaf
	{
		_emb, err := codec_origin_leaf.DecodeCodecOriginLeaf(cursor)
		if err != nil {
			return nil, err
		}
		Required = *_emb
	}
	var Optional *codec_origin_leaf.CodecOriginLeaf
	if (Hdr & 0x01) != 0 {
		_emb, err := codec_origin_leaf.DecodeCodecOriginLeaf(cursor)
		if err != nil {
			return nil, err
		}
		Optional = _emb
	}
	Items := make([]codec_origin_leaf.CodecOriginLeaf, 0, M)
	for _i := 0; _i < int(M); _i++ {
		_elem, err := codec_origin_leaf.DecodeCodecOriginLeaf(cursor)
		if err != nil {
			return nil, err
		}
		Items = append(Items, *_elem)
	}
	return &CodecOriginEnvelope{
		Hdr: Hdr,
		NoteLen: NoteLen,
		Note: Note,
		M: M,
		Required: Required,
		Optional: Optional,
		Items: Items,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecOriginEnvelope) HasOpt() bool {
	return (s.Hdr & 0x01) != 0
}

func (s *CodecOriginEnvelope) SetHasOpt(v bool) {
	if v {
		s.Hdr |= 0x01
	} else {
		s.Hdr &^= 0x01
	}
}

// Encode writes the CodecOriginEnvelope into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecOriginEnvelope) Encode(w codec.SceSink) error {
	// Streaming cursor encode (SSOT selection: `needs_streaming`).
	// Mirrors the streaming decode: every field appends its own bytes in
	// declaration order through the per-field encode blocks, so a gated
	// field skips its append when absent, and a fixed field after a
	// variable-length payload lands after the payload (the positional path
	// appends variable fields last, placing it ahead on the wire).
	// Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
	// dedicated helpers; everything else uses `present_if_encode_block`.
	if err := w.WriteBytes([]byte{ s.Hdr }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ s.NoteLen }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte(s.Note)); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ s.M }); err != nil {
		return err
	}
	if err := s.Required.Encode(w); err != nil {
		return err
	}
	if s.Optional != nil {
		if err := s.Optional.Encode(w); err != nil {
			return err
		}
	}
	for _i := range s.Items {
		if err := s.Items[_i].Encode(w); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecOriginEnvelope) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 583)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
