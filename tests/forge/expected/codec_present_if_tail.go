// SCE-MAP: codec_present_if_tail:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_present_if_tail

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
)

// CodecPresentIfTailPayloadBound is the bound of the payload element of CodecPresentIfTail: 0 to 64 bytes
// (docs/adr/0077 §3). A Bytes of it is made only within it.
type CodecPresentIfTailPayloadBound struct{}

// Least is the fewest bytes the element holds.
func (CodecPresentIfTailPayloadBound) Least() int { return 0 }

// Most is the most bytes the element holds.
func (CodecPresentIfTailPayloadBound) Most() int { return 64 }

// CodecPresentIfTailSource is what the encode of CodecPresentIfTail reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecPresentIfTail is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecPresentIfTailSource interface {
	Flags() uint8
	Payload() optional.Of[codec.Bytes[CodecPresentIfTailPayloadBound]]
}

// CodecPresentIfTail represents the codec frame layout.
type CodecPresentIfTail struct {
	flags uint8
	payload optional.Of[codec.Bytes[CodecPresentIfTailPayloadBound]]
}

// MakeCodecPresentIfTail returns the CodecPresentIfTail that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecPresentIfTail, or as its zero value, and is read through its methods.
func MakeCodecPresentIfTail(flags uint8, payload optional.Of[codec.Bytes[CodecPresentIfTailPayloadBound]]) CodecPresentIfTail {
	return CodecPresentIfTail{flags: flags, payload: payload}
}

// Flags returns the element the value presents as Flags.
func (s *CodecPresentIfTail) Flags() uint8 {
	return s.flags
}

// Payload returns the element the value presents as Payload.
func (s *CodecPresentIfTail) Payload() optional.Of[codec.Bytes[CodecPresentIfTailPayloadBound]] {
	return s.payload
}

// EncodeSource writes any source of CodecPresentIfTail into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecPresentIfTailSource, w codec.SceSink) error {
	_cFlags := src.Flags()
	_cFlags &^= 0x1
	if src.Payload().IsPresent() {
		_cFlags |= 0x1
	}
	if err := w.WriteBytes([]byte{byte(_cFlags)}); err != nil {
		return err
	}
	if src.Payload().IsPresent() {
		if err := w.WriteBytes(src.Payload().Proven().Bytes()); err != nil {
			return err
		}
	}
	return nil
}

// DecodeCodecPresentIfTail decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecPresentIfTail(cursor *codec.SceCursor) (*CodecPresentIfTail, error) {
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
	var Payload optional.Of[[]byte]
	if (Flags & 0x01) != 0 {
		_n := cursor.Remaining()
		if _n > 64 {
			return nil, codec.ErrLengthOutOfRange
		}
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Payload = optional.Some(append([]byte(nil), raw...))
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	_bPayload, err := codec.MakeOptionalBytes[CodecPresentIfTailPayloadBound](Payload)
	if err != nil {
		return nil, err
	}
	return &CodecPresentIfTail{
		flags: Flags,
		payload: _bPayload,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecPresentIfTail) HasPayload() bool {
	return (s.flags & 0x01) != 0
}

func (s *CodecPresentIfTail) SetHasPayload(v bool) {
	if v {
		s.flags |= 0x01
	} else {
		s.flags &^= 0x01
	}
}

// Encode writes the CodecPresentIfTail into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecPresentIfTail) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecPresentIfTail) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 65)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
