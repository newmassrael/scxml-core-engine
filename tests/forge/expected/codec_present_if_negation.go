// SCE-MAP: codec_present_if_negation:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_present_if_negation

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
)

// CodecPresentIfNegationSource is what the encode of CodecPresentIfNegation reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecPresentIfNegation is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecPresentIfNegationSource interface {
	Flags() uint8
	Seq() optional.Of[uint16]
}

// CodecPresentIfNegation represents the codec frame layout.
type CodecPresentIfNegation struct {
	flags uint8
	seq optional.Of[uint16]
}

// MakeCodecPresentIfNegation returns the CodecPresentIfNegation that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecPresentIfNegation, or as its zero value, and is read through its methods.
func MakeCodecPresentIfNegation(flags uint8, seq optional.Of[uint16]) CodecPresentIfNegation {
	return CodecPresentIfNegation{flags: flags, seq: seq}
}

// Flags returns the element the value presents as Flags.
func (s *CodecPresentIfNegation) Flags() uint8 {
	return s.flags
}

// Seq returns the element the value presents as Seq.
func (s *CodecPresentIfNegation) Seq() optional.Of[uint16] {
	return s.seq
}

// EncodeSource writes any source of CodecPresentIfNegation into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecPresentIfNegationSource, w codec.SceSink) error {
	_cFlags := src.Flags()
	_cFlags &^= 0x1
	if !src.Seq().IsPresent() {
		_cFlags |= 0x1
	}
	if err := w.WriteBytes([]byte{byte(_cFlags)}); err != nil {
		return err
	}
	if src.Seq().IsPresent() {
		if err := w.WriteBytes([]byte{byte(src.Seq().Proven() >> 8 & 0xFF), byte(src.Seq().Proven() & 0xFF)}); err != nil {
			return err
		}
	}
	return nil
}

// DecodeCodecPresentIfNegation decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecPresentIfNegation(cursor *codec.SceCursor) (*CodecPresentIfNegation, error) {
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
	var Flags uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Flags = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var Seq optional.Of[uint16]
	if (Flags & 0x01) == 0 {
		raw, err := cursor.PeekSlice(2)
		if err != nil {
			return nil, err
		}
		_v := uint16(raw[0])<<8 | uint16(raw[1])
		if err := cursor.Advance(2); err != nil {
			return nil, err
		}
		Seq = optional.Some(_v)
	}
	return &CodecPresentIfNegation{
		flags: Flags,
		seq: Seq,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecPresentIfNegation) AbsentSeq() bool {
	return (s.flags & 0x01) != 0
}

func (s *CodecPresentIfNegation) SetAbsentSeq(v bool) {
	if v {
		s.flags |= 0x01
	} else {
		s.flags &^= 0x01
	}
}

// Encode writes the CodecPresentIfNegation into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecPresentIfNegation) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecPresentIfNegation) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 3)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
