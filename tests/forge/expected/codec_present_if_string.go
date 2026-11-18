// SCE-MAP: codec_present_if_string:47 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_present_if_string

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
	"unicode/utf8"
)

// CodecPresentIfStringTextBound is the bound of the text element of CodecPresentIfString: 0 to 32 UTF-8 bytes
// (docs/adr/0077 §3). A Text of it is made only within it.
type CodecPresentIfStringTextBound struct{}

// Least is the fewest UTF-8 bytes the element holds.
func (CodecPresentIfStringTextBound) Least() int { return 0 }

// Most is the most UTF-8 bytes the element holds.
func (CodecPresentIfStringTextBound) Most() int { return 32 }

// CodecPresentIfStringSource is what the encode of CodecPresentIfString reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecPresentIfString is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecPresentIfStringSource interface {
	Carrier() uint8
	Text() optional.Of[codec.Text[CodecPresentIfStringTextBound]]
}

// CodecPresentIfString represents the codec frame layout.
type CodecPresentIfString struct {
	carrier uint8
	text optional.Of[codec.Text[CodecPresentIfStringTextBound]]
}

// MakeCodecPresentIfString returns the CodecPresentIfString that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecPresentIfString, or as its zero value, and is read through its methods.
func MakeCodecPresentIfString(carrier uint8, text optional.Of[codec.Text[CodecPresentIfStringTextBound]]) CodecPresentIfString {
	return CodecPresentIfString{carrier: carrier, text: text}
}

// Carrier returns the element the value presents as Carrier.
func (s *CodecPresentIfString) Carrier() uint8 {
	return s.carrier
}

// Text returns the element the value presents as Text.
func (s *CodecPresentIfString) Text() optional.Of[codec.Text[CodecPresentIfStringTextBound]] {
	return s.text
}

// WireTextLen returns the text_len the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecPresentIfString) WireTextLen() optional.Of[uint8] {
	if _v, ok := s.Text().Get(); ok {
		return optional.Some(uint8(_v.Len()))
	}
	return optional.None[uint8]()
}

// EncodeSource writes any source of CodecPresentIfString into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives text_len from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecPresentIfStringSource, w codec.SceSink) error {
	var _dTextLen uint8
	if src.Text().IsPresent() {
		_dTextLen = uint8(src.Text().Proven().Len())
	}
	_cCarrier := src.Carrier()
	_cCarrier &^= 0x1
	if src.Text().IsPresent() {
		_cCarrier |= 0x1
	}
	if err := w.WriteBytes([]byte{byte(_cCarrier)}); err != nil {
		return err
	}
	if ((_cCarrier >> 0) & 1) != 0 {
		if err := w.WriteBytes([]byte{byte(_dTextLen)}); err != nil {
			return err
		}
	}
	if src.Text().IsPresent() {
		if err := w.WriteBytes([]byte(src.Text().Proven().String())); err != nil {
			return err
		}
	}
	return nil
}

// DecodeCodecPresentIfString decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecPresentIfString(cursor *codec.SceCursor) (*CodecPresentIfString, error) {
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
	var TextLen optional.Of[uint8]
	if (Carrier & 0x01) != 0 {
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		_v := raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
		TextLen = optional.Some(_v)
	}
	var Text optional.Of[string]
	if (Carrier & 0x01) != 0 {
		_len := TextLen.Proven()
		if _len > 32 {
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
		Text = optional.Some(string(raw))
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	_bText, err := codec.MakeOptionalText[CodecPresentIfStringTextBound](Text)
	if err != nil {
		return nil, err
	}
	return &CodecPresentIfString{
		carrier: Carrier,
		text: _bText,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecPresentIfString) HasText() bool {
	return (s.carrier & 0x01) != 0
}

func (s *CodecPresentIfString) SetHasText(v bool) {
	if v {
		s.carrier |= 0x01
	} else {
		s.carrier &^= 0x01
	}
}

// Encode writes the CodecPresentIfString into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecPresentIfString) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
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
