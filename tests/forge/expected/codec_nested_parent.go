// SCE-MAP: codec_nested_parent:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_nested_parent

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
	"example.com/sce-forge/codec_nested_body"
)

// CodecNestedParentBodyListBound is the bound of the body_list element of CodecNestedParent: 0 to 4 entries
// (docs/adr/0065 §6 R2). A List of it is made only within it.
type CodecNestedParentBodyListBound struct{}

// Least is the fewest entries the element holds.
func (CodecNestedParentBodyListBound) Least() int { return 0 }

// Most is the most entries the element holds.
func (CodecNestedParentBodyListBound) Most() int { return 4 }

// CodecNestedParentSource is what the encode of CodecNestedParent reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecNestedParent is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecNestedParentSource interface {
	Hdr() uint8
	RequiredBody() codec_nested_body.CodecNestedBodySource
	OptionalBody() optional.Of[codec_nested_body.CodecNestedBodySource]
	BodyList() codec.List[CodecNestedParentBodyListBound, codec_nested_body.CodecNestedBodySource]
}

// CodecNestedParent represents the codec frame layout.
type CodecNestedParent struct {
	hdr uint8
	requiredBody codec_nested_body.CodecNestedBodySource
	optionalBody optional.Of[codec_nested_body.CodecNestedBodySource]
	bodyList codec.List[CodecNestedParentBodyListBound, codec_nested_body.CodecNestedBodySource]
}

// MakeCodecNestedParent returns the CodecNestedParent that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecNestedParent, or as its zero value, and is read through its methods.
func MakeCodecNestedParent(hdr uint8, requiredBody codec_nested_body.CodecNestedBodySource, optionalBody optional.Of[codec_nested_body.CodecNestedBodySource], bodyList codec.List[CodecNestedParentBodyListBound, codec_nested_body.CodecNestedBodySource]) CodecNestedParent {
	return CodecNestedParent{hdr: hdr, requiredBody: requiredBody, optionalBody: optionalBody, bodyList: bodyList}
}

// Hdr returns the element the value presents as Hdr.
func (s *CodecNestedParent) Hdr() uint8 {
	return s.hdr
}

// RequiredBody returns the element the value presents as RequiredBody.
func (s *CodecNestedParent) RequiredBody() codec_nested_body.CodecNestedBodySource {
	if s.requiredBody == nil {
		return &codec_nested_body.CodecNestedBody{}
	}
	return s.requiredBody
}

// OptionalBody returns the element the value presents as OptionalBody.
func (s *CodecNestedParent) OptionalBody() optional.Of[codec_nested_body.CodecNestedBodySource] {
	return s.optionalBody
}

// BodyList returns the element the value presents as BodyList.
func (s *CodecNestedParent) BodyList() codec.List[CodecNestedParentBodyListBound, codec_nested_body.CodecNestedBodySource] {
	return s.bodyList
}

// WireM returns the m the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecNestedParent) WireM() uint8 {
	return uint8(s.BodyList().Len())
}

// EncodeSource writes any source of CodecNestedParent into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives m from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecNestedParentSource, w codec.SceSink) error {
	_dM := uint8(src.BodyList().Len())
	_cHdr := src.Hdr()
	_cHdr &^= 0x1
	if src.OptionalBody().IsPresent() {
		_cHdr |= 0x1
	}
	if err := w.WriteBytes([]byte{byte(_cHdr)}); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{byte(_dM)}); err != nil {
		return err
	}
	if err := codec_nested_body.EncodeSource(src.RequiredBody(), w); err != nil {
		return err
	}
	if src.OptionalBody().IsPresent() {
		if err := codec_nested_body.EncodeSource(src.OptionalBody().Proven(), w); err != nil {
			return err
		}
	}
	for _e := range src.BodyList().All() {
		if err := codec_nested_body.EncodeSource(_e, w); err != nil {
			return err
		}
	}
	return nil
}

// DecodeCodecNestedParent decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecNestedParent(cursor *codec.SceCursor) (*CodecNestedParent, error) {
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
	var RequiredBody codec_nested_body.CodecNestedBody
	{
		_emb, err := codec_nested_body.DecodeCodecNestedBody(cursor)
		if err != nil {
			return nil, err
		}
		RequiredBody = *_emb
	}
	var OptionalBody optional.Of[codec_nested_body.CodecNestedBody]
	if (Hdr & 0x01) != 0 {
		_emb, err := codec_nested_body.DecodeCodecNestedBody(cursor)
		if err != nil {
			return nil, err
		}
		OptionalBody = optional.Some(*_emb)
	}
	if M > 4 {
		return nil, codec.ErrTooManyElements
	}
	BodyList := make([]codec_nested_body.CodecNestedBody, 0, cursor.EntriesBacked(uint64(M)))
	for _i := 0; _i < int(M); _i++ {
		_elem, err := codec_nested_body.DecodeCodecNestedBody(cursor)
		if err != nil {
			return nil, err
		}
		BodyList = append(BodyList, *_elem)
	}
	_lBodyList, err := codec.MakeList[CodecNestedParentBodyListBound](codec.SourcesOf(BodyList, func(e *codec_nested_body.CodecNestedBody) codec_nested_body.CodecNestedBodySource { return e }))
	if err != nil {
		return nil, err
	}
	return &CodecNestedParent{
		hdr: Hdr,
		requiredBody: &RequiredBody,
		optionalBody: optional.Convert(OptionalBody, func(b codec_nested_body.CodecNestedBody) codec_nested_body.CodecNestedBodySource { return &b }),
		bodyList: _lBodyList,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecNestedParent) HasOpt() bool {
	return (s.hdr & 0x01) != 0
}

func (s *CodecNestedParent) SetHasOpt(v bool) {
	if v {
		s.hdr |= 0x01
	} else {
		s.hdr &^= 0x01
	}
}

// Encode writes the CodecNestedParent into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecNestedParent) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecNestedParent) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 6344)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
