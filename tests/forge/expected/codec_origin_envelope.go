// SCE-MAP: codec_origin_envelope:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_origin_envelope

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
	"unicode/utf8"
	"example.com/sce-forge/codec_origin_leaf"
)

// CodecOriginEnvelopeNoteBound is the bound of the note element of CodecOriginEnvelope: 0 to 16 UTF-8 bytes
// (docs/adr/0077 §3). A Text of it is made only within it.
type CodecOriginEnvelopeNoteBound struct{}

// Least is the fewest UTF-8 bytes the element holds.
func (CodecOriginEnvelopeNoteBound) Least() int { return 0 }

// Most is the most UTF-8 bytes the element holds.
func (CodecOriginEnvelopeNoteBound) Most() int { return 16 }

// CodecOriginEnvelopeItemsBound is the bound of the items element of CodecOriginEnvelope: 0 to 3 entries
// (docs/adr/0065 §6 R2). A List of it is made only within it.
type CodecOriginEnvelopeItemsBound struct{}

// Least is the fewest entries the element holds.
func (CodecOriginEnvelopeItemsBound) Least() int { return 0 }

// Most is the most entries the element holds.
func (CodecOriginEnvelopeItemsBound) Most() int { return 3 }

// CodecOriginEnvelopeSource is what the encode of CodecOriginEnvelope reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecOriginEnvelope is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecOriginEnvelopeSource interface {
	Hdr() uint8
	Note() codec.Text[CodecOriginEnvelopeNoteBound]
	Required() codec_origin_leaf.CodecOriginLeafSource
	Optional() optional.Of[codec_origin_leaf.CodecOriginLeafSource]
	Items() codec.List[CodecOriginEnvelopeItemsBound, codec_origin_leaf.CodecOriginLeafSource]
}

// CodecOriginEnvelope represents the codec frame layout.
type CodecOriginEnvelope struct {
	hdr uint8
	note codec.Text[CodecOriginEnvelopeNoteBound]
	required codec_origin_leaf.CodecOriginLeafSource
	optional optional.Of[codec_origin_leaf.CodecOriginLeafSource]
	items codec.List[CodecOriginEnvelopeItemsBound, codec_origin_leaf.CodecOriginLeafSource]
}

// MakeCodecOriginEnvelope returns the CodecOriginEnvelope that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecOriginEnvelope, or as its zero value, and is read through its methods.
func MakeCodecOriginEnvelope(hdr uint8, note codec.Text[CodecOriginEnvelopeNoteBound], required codec_origin_leaf.CodecOriginLeafSource, optional optional.Of[codec_origin_leaf.CodecOriginLeafSource], items codec.List[CodecOriginEnvelopeItemsBound, codec_origin_leaf.CodecOriginLeafSource]) CodecOriginEnvelope {
	return CodecOriginEnvelope{hdr: hdr, note: note, required: required, optional: optional, items: items}
}

// Hdr returns the element the value presents as Hdr.
func (s *CodecOriginEnvelope) Hdr() uint8 {
	return s.hdr
}

// Note returns the element the value presents as Note.
func (s *CodecOriginEnvelope) Note() codec.Text[CodecOriginEnvelopeNoteBound] {
	return s.note
}

// Required returns the element the value presents as Required.
func (s *CodecOriginEnvelope) Required() codec_origin_leaf.CodecOriginLeafSource {
	if s.required == nil {
		return &codec_origin_leaf.CodecOriginLeaf{}
	}
	return s.required
}

// Optional returns the element the value presents as Optional.
func (s *CodecOriginEnvelope) Optional() optional.Of[codec_origin_leaf.CodecOriginLeafSource] {
	return s.optional
}

// Items returns the element the value presents as Items.
func (s *CodecOriginEnvelope) Items() codec.List[CodecOriginEnvelopeItemsBound, codec_origin_leaf.CodecOriginLeafSource] {
	return s.items
}

// WireNoteLen returns the note_len the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecOriginEnvelope) WireNoteLen() uint8 {
	return uint8(s.Note().Len())
}

// WireM returns the m the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecOriginEnvelope) WireM() uint8 {
	return uint8(s.Items().Len())
}

// EncodeSource writes any source of CodecOriginEnvelope into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives m, note_len from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecOriginEnvelopeSource, w codec.SceSink) error {
	_dNoteLen := uint8(src.Note().Len())
	_dM := uint8(src.Items().Len())
	_cHdr := src.Hdr()
	_cHdr &^= 0x1
	if src.Optional().IsPresent() {
		_cHdr |= 0x1
	}
	if err := w.WriteBytes([]byte{byte(_cHdr)}); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{byte(_dNoteLen)}); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte(src.Note().String())); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{byte(_dM)}); err != nil {
		return err
	}
	if err := codec_origin_leaf.EncodeSource(src.Required(), w); err != nil {
		return err
	}
	if src.Optional().IsPresent() {
		if err := codec_origin_leaf.EncodeSource(src.Optional().Proven(), w); err != nil {
			return err
		}
	}
	for _e := range src.Items().All() {
		if err := codec_origin_leaf.EncodeSource(_e, w); err != nil {
			return err
		}
	}
	return nil
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
	// (runtime presence, held by the runtime's optional type), VLE / repeat / TLV-chain /
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
		_len := NoteLen
		if _len > 16 {
			return nil, codec.ErrLengthOutOfRange
		}
		_n := int(_len)
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
	var Optional optional.Of[codec_origin_leaf.CodecOriginLeaf]
	if (Hdr & 0x01) != 0 {
		_emb, err := codec_origin_leaf.DecodeCodecOriginLeaf(cursor)
		if err != nil {
			return nil, err
		}
		Optional = optional.Some(*_emb)
	}
	if M > 3 {
		return nil, codec.ErrTooManyElements
	}
	Items := make([]codec_origin_leaf.CodecOriginLeaf, 0, cursor.EntriesBacked(uint64(M)))
	for _i := 0; _i < int(M); _i++ {
		_elem, err := codec_origin_leaf.DecodeCodecOriginLeaf(cursor)
		if err != nil {
			return nil, err
		}
		Items = append(Items, *_elem)
	}
	_bNote, err := codec.MakeText[CodecOriginEnvelopeNoteBound](string(Note))
	if err != nil {
		return nil, err
	}
	_lItems, err := codec.MakeList[CodecOriginEnvelopeItemsBound](codec.SourcesOf(Items, func(e *codec_origin_leaf.CodecOriginLeaf) codec_origin_leaf.CodecOriginLeafSource { return e }))
	if err != nil {
		return nil, err
	}
	return &CodecOriginEnvelope{
		hdr: Hdr,
		note: _bNote,
		required: &Required,
		optional: optional.Convert(Optional, func(b codec_origin_leaf.CodecOriginLeaf) codec_origin_leaf.CodecOriginLeafSource { return &b }),
		items: _lItems,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecOriginEnvelope) HasOpt() bool {
	return (s.hdr & 0x01) != 0
}

func (s *CodecOriginEnvelope) SetHasOpt(v bool) {
	if v {
		s.hdr |= 0x01
	} else {
		s.hdr &^= 0x01
	}
}

// Encode writes the CodecOriginEnvelope into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecOriginEnvelope) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecOriginEnvelope) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 105)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
