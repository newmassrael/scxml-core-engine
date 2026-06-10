// SCE-MAP: codec_present_if_string:47

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_present_if_string

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"unicode/utf8"
)

// CodecPresentIfString represents the codec frame layout.
type CodecPresentIfString struct {
	Carrier uint8
	TextLen *uint8
	Text *string
}

// DecodeCodecPresentIfString decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecPresentIfString(cursor *codec.SceCursor) (*CodecPresentIfString, error) {
	// RFC §synth-5-B present-if primitive: streaming decode
	// advances the cursor per field. Gated fields use `*T` for fixed
	// (nil = absent) or `[]byte` (nil = absent) for tail/length-ref;
	// VLE gating uses `*T` like fixed. Per-field `is_repeat` routes
	// Repeat fields to the dedicated helper. Branch fires before
	// has_vle_fields so a codec mixing VLE + present-if uses the
	// unified streaming path.
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
	var TextLen *uint8
	if (Carrier & 0x01) != 0 {
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		_v := raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
		TextLen = &_v
	}
	var Text *string
	if (Carrier & 0x01) != 0 {
		_n := int(*TextLen)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		if !utf8.Valid(raw) {
			return nil, codec.ErrInvalidUTF8
		}
		_v := string(raw)
		Text = &_v
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	return &CodecPresentIfString{
		Carrier: Carrier,
		TextLen: TextLen,
		Text: Text,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecPresentIfString) HasText() bool {
	return (s.Carrier & 0x01) != 0
}

func (s *CodecPresentIfString) SetHasText(v bool) {
	if v {
		s.Carrier |= 0x01
	} else {
		s.Carrier &^= 0x01
	}
}

// Encode writes the CodecPresentIfString into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecPresentIfString) Encode(w codec.SceSink) error {
	// RFC §synth-5-B present-if encode.
	if err := w.WriteBytes([]byte{ s.Carrier }); err != nil {
		return err
	}
	if s.TextLen != nil {
		_v := *s.TextLen
		if err := w.WriteBytes([]byte{ _v }); err != nil {
			return err
		}
	}
	if s.Text != nil {
		if err := w.WriteBytes([]byte(*s.Text)); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecPresentIfString) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 34)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
