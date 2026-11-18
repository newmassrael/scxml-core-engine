// SCE-MAP: codec_ext_attachment:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_ext_attachment

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecExtAttachmentBodyBound is the bound of the body element of CodecExtAttachment: 0 to 64 bytes
// (docs/adr/0077 §3). A Bytes of it is made only within it.
type CodecExtAttachmentBodyBound struct{}

// Least is the fewest bytes the element holds.
func (CodecExtAttachmentBodyBound) Least() int { return 0 }

// Most is the most bytes the element holds.
func (CodecExtAttachmentBodyBound) Most() int { return 64 }

// CodecExtAttachmentSource is what the encode of CodecExtAttachment reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecExtAttachment is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecExtAttachmentSource interface {
	Body() codec.Bytes[CodecExtAttachmentBodyBound]
}

// CodecExtAttachment represents the codec frame layout.
type CodecExtAttachment struct {
	body codec.Bytes[CodecExtAttachmentBodyBound]
}

// MakeCodecExtAttachment returns the CodecExtAttachment that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecExtAttachment, or as its zero value, and is read through its methods.
func MakeCodecExtAttachment(body codec.Bytes[CodecExtAttachmentBodyBound]) CodecExtAttachment {
	return CodecExtAttachment{body: body}
}

// Body returns the element the value presents as Body.
func (s *CodecExtAttachment) Body() codec.Bytes[CodecExtAttachmentBodyBound] {
	return s.body
}

// WireLength returns the length the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecExtAttachment) WireLength() uint8 {
	return uint8(s.Body().Len())
}

// EncodeSource writes any source of CodecExtAttachment into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives length from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecExtAttachmentSource, w codec.SceSink) error {
	_dLength := uint8(src.Body().Len())
	if err := w.WriteBytes([]byte{byte(_dLength)}); err != nil {
		return err
	}
	if err := w.WriteBytes(src.Body().Bytes()); err != nil {
		return err
	}
	return nil
}

// DecodeCodecExtAttachment decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecExtAttachment(cursor *codec.SceCursor) (*CodecExtAttachment, error) {
	frameLen := cursor.Remaining()
	if frameLen < 1 {
		return nil, codec.ErrNeedMoreBytes
	}
	raw, err := cursor.PeekSlice(frameLen)
	if err != nil {
		return nil, err
	}
	Length := raw[0]
	if Length > 64 {
		return nil, codec.ErrLengthOutOfRange
	}
	if frameLen < 1+int(Length) {
		return nil, codec.ErrNeedMoreBytes
	}
	Body := raw[1:1+int(Length)]
	_bBody, err := codec.MakeBytes[CodecExtAttachmentBodyBound](Body)
	if err != nil {
		return nil, err
	}
	value := &CodecExtAttachment{
		body: _bBody,
	}
	// The bytes it read and no more (RFC §synth-5-B B3, as the Rust decode
	// does): the end of the last length-ref field, at least the fixed fields'
	// bytes, so that a codec holding this one as a body, an entry or an arm
	// decodes on after it. Each end is an argument of `max`, where gofmt
	// spells it as the guard above does.
	consumed := 1
	consumed = max(consumed, 1+int(Length))
	if err := cursor.Advance(consumed); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode writes the CodecExtAttachment into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecExtAttachment) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecExtAttachment) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 65)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
