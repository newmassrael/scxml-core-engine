// SCE-MAP: codec_zenoh_frame:25 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_frame

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohFramePayloadBound is the bound of the payload element of CodecZenohFrame: 0 to 65536 bytes
// (docs/adr/0077 §3). A Bytes of it is made only within it.
type CodecZenohFramePayloadBound struct{}

// Least is the fewest bytes the element holds.
func (CodecZenohFramePayloadBound) Least() int { return 0 }

// Most is the most bytes the element holds.
func (CodecZenohFramePayloadBound) Most() int { return 65536 }

// CodecZenohFrameSource is what the encode of CodecZenohFrame reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohFrame is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohFrameSource interface {
	Sn() uint64
	Payload() codec.Bytes[CodecZenohFramePayloadBound]
}

// CodecZenohFrame represents the codec frame layout.
type CodecZenohFrame struct {
	sn uint64
	payload codec.Bytes[CodecZenohFramePayloadBound]
}

// MakeCodecZenohFrame returns the CodecZenohFrame that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohFrame, or as its zero value, and is read through its methods.
func MakeCodecZenohFrame(sn uint64, payload codec.Bytes[CodecZenohFramePayloadBound]) CodecZenohFrame {
	return CodecZenohFrame{sn: sn, payload: payload}
}

// Sn returns the element the value presents as Sn.
func (s *CodecZenohFrame) Sn() uint64 {
	return s.sn
}

// Payload returns the element the value presents as Payload.
func (s *CodecZenohFrame) Payload() codec.Bytes[CodecZenohFramePayloadBound] {
	return s.payload
}

// EncodeSource writes any source of CodecZenohFrame into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohFrameSource, w codec.SceSink) error {
	if err := codec.WriteVLEU64(w, uint64(src.Sn())); err != nil {
		return err
	}
	if err := w.WriteBytes(src.Payload().Bytes()); err != nil {
		return err
	}
	return nil
}

// DecodeCodecZenohFrame decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohFrame(cursor *codec.SceCursor) (*CodecZenohFrame, error) {
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
	Sn, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var Payload []byte
	{
		_n := cursor.Remaining()
		if _n > 65536 {
			return nil, codec.ErrLengthOutOfRange
		}
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Payload = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	_bPayload, err := codec.MakeBytes[CodecZenohFramePayloadBound](Payload)
	if err != nil {
		return nil, err
	}
	return &CodecZenohFrame{
		sn: Sn,
		payload: _bPayload,
	}, nil
}

// Encode writes the CodecZenohFrame into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohFrame) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohFrame) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 65545)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
