// SCE-MAP: codec_origin_choice:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_origin_choice

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_origin_leaf"
	"example.com/sce-forge/codec_origin_scalar"
)

// CodecOriginChoiceDefault bundles the runtime
// tag value with the catch-all body so encode can round-trip the
// observed tag back onto the wire (RFC §synth-5-B variant primitive).
type CodecOriginChoiceDefault struct {
	Tag uint8
	Body codec_origin_scalar.CodecOriginScalar
}

// CodecOriginChoiceVariant is a discriminated-union body for the codec's
// tag-field suffix (RFC §synth-5-B variant primitive). Exactly one of
// the pointer fields is non-nil at a time; the active arm is the one
// that matches the current tag value.
type CodecOriginChoiceVariant struct {
	CodecOriginLeaf *codec_origin_leaf.CodecOriginLeaf
	CodecOriginScalar *codec_origin_scalar.CodecOriginScalar
	Default *CodecOriginChoiceDefault
}

// CodecOriginChoice represents the codec frame layout.
type CodecOriginChoice struct {
	Tag uint8
	Body CodecOriginChoiceVariant
}

// NewCodecOriginChoice returns a CodecOriginChoice initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecOriginChoice().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecOriginChoice{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecOriginChoice() *CodecOriginChoice {
	return &CodecOriginChoice{
		Body: CodecOriginChoiceVariant{
			CodecOriginScalar: &codec_origin_scalar.CodecOriginScalar{},
		},
	}
}

// DecodeCodecOriginChoice decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecOriginChoice(cursor *codec.SceCursor) (*CodecOriginChoice, error) {
	// Decode fixed prefix (RFC §synth-5-B variant: fields before tag suffix).
	raw, err := cursor.PeekSlice(1)
	if err != nil {
		return nil, err
	}
	Tag := raw[0]
	if err := cursor.Advance(1); err != nil {
		return nil, err
	}
	// Dispatch on the tag field; each arm decodes its body codec from
	// the cursor. The default arm (when declared) carries the runtime
	// tag value so encode can round-trip it back onto the wire.
	body := CodecOriginChoiceVariant{}
	switch Tag {
	case 1:
		_arm, err := codec_origin_leaf.DecodeCodecOriginLeaf(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecOriginLeaf = _arm
	case 2:
		_arm, err := codec_origin_scalar.DecodeCodecOriginScalar(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecOriginScalar = _arm
	default:
		_arm, err := codec_origin_scalar.DecodeCodecOriginScalar(cursor)
		if err != nil {
			return nil, err
		}
		body.Default = &CodecOriginChoiceDefault{
			Tag: Tag,
			Body: *_arm,
		}
	}
	return &CodecOriginChoice{
		Tag: Tag,
		Body: body,
	}, nil
}

// Encode writes the CodecOriginChoice into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecOriginChoice) Encode(w codec.SceSink) error {
	// Encode fixed prefix (tag field bytes are part of the prefix).
	if err := w.WriteBytes([]byte{ byte(s.Tag) }); err != nil {
		return err
	}
	// Append the active arm body's encoded bytes via the same sink.
	switch {
	case s.Body.CodecOriginLeaf != nil:
		if err := s.Body.CodecOriginLeaf.Encode(w); err != nil {
			return err
		}
	case s.Body.CodecOriginScalar != nil:
		if err := s.Body.CodecOriginScalar.Encode(w); err != nil {
			return err
		}
	case s.Body.Default != nil:
		if err := s.Body.Default.Body.Encode(w); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecOriginChoice) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 18)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
