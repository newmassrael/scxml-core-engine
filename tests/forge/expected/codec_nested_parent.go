// SCE-MAP: codec_nested_parent:22

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_nested_parent

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_nested_body"
)

// CodecNestedParent represents the codec frame layout.
type CodecNestedParent struct {
	Hdr uint8
	M uint8
	RequiredBody codec_nested_body.CodecNestedBody
	OptionalBody *codec_nested_body.CodecNestedBody
	BodyList []codec_nested_body.CodecNestedBody
}

// DecodeCodecNestedParent decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecNestedParent(cursor *codec.SceCursor) (*CodecNestedParent, error) {
	// RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
	// advances the cursor per field. Gated fields use `*T` for fixed
	// (nil = absent) or `[]byte` (nil = absent) for tail/length-ref;
	// VLE gating uses `*T` like fixed. Per-field `is_repeat` routes
	// Repeat fields to the dedicated helper. Branch fires before
	// has_vle_fields so a codec mixing VLE + present-if uses the
	// unified streaming path.
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
	var OptionalBody *codec_nested_body.CodecNestedBody
	if (Hdr & 0x01) != 0 {
		_emb, err := codec_nested_body.DecodeCodecNestedBody(cursor)
		if err != nil {
			return nil, err
		}
		OptionalBody = _emb
	}
	BodyList := make([]codec_nested_body.CodecNestedBody, 0, M)
	for _i := 0; _i < int(M); _i++ {
		_elem, err := codec_nested_body.DecodeCodecNestedBody(cursor)
		if err != nil {
			return nil, err
		}
		BodyList = append(BodyList, *_elem)
	}
	return &CodecNestedParent{
		Hdr: Hdr,
		M: M,
		RequiredBody: RequiredBody,
		OptionalBody: OptionalBody,
		BodyList: BodyList,
	}, nil
}

// RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecNestedParent) HasOpt() bool {
	return (s.Hdr & 0x01) != 0
}

func (s *CodecNestedParent) SetHasOpt(v bool) {
	if v {
		s.Hdr |= 0x01
	} else {
		s.Hdr &^= 0x01
	}
}

// Encode writes the CodecNestedParent into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecNestedParent) Encode(w codec.SceSink) error {
	// RFC §5.B B1-δ + B2-β present-if encode.
	if err := w.WriteBytes([]byte{ s.Hdr }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ s.M }); err != nil {
		return err
	}
	if err := s.RequiredBody.Encode(w); err != nil {
		return err
	}
	if s.OptionalBody != nil {
		if err := s.OptionalBody.Encode(w); err != nil {
			return err
		}
	}
	for _i := range s.BodyList {
		if err := s.BodyList[_i].Encode(w); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecNestedParent) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 2726)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
