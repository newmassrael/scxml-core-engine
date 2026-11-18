// SCE-MAP: codec_zenoh_fragment:23 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_fragment

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohFragmentPayloadBound is the bound of the payload element of CodecZenohFragment: 0 to 65536 bytes
// (docs/adr/0077 §3). A Bytes of it is made only within it.
type CodecZenohFragmentPayloadBound struct{}

// Least is the fewest bytes the element holds.
func (CodecZenohFragmentPayloadBound) Least() int { return 0 }

// Most is the most bytes the element holds.
func (CodecZenohFragmentPayloadBound) Most() int { return 65536 }

// CodecZenohFragmentSource is what the encode of CodecZenohFragment reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohFragment is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohFragmentSource interface {
	Sn() uint64
	Payload() codec.Bytes[CodecZenohFragmentPayloadBound]
}

// CodecZenohFragment represents the codec frame layout.
type CodecZenohFragment struct {
	sn uint64
	payload codec.Bytes[CodecZenohFragmentPayloadBound]
}

// MakeCodecZenohFragment returns the CodecZenohFragment that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohFragment, or as its zero value, and is read through its methods.
func MakeCodecZenohFragment(sn uint64, payload codec.Bytes[CodecZenohFragmentPayloadBound]) CodecZenohFragment {
	return CodecZenohFragment{sn: sn, payload: payload}
}

// Sn returns the element the value presents as Sn.
func (s *CodecZenohFragment) Sn() uint64 {
	return s.sn
}

// Payload returns the element the value presents as Payload.
func (s *CodecZenohFragment) Payload() codec.Bytes[CodecZenohFragmentPayloadBound] {
	return s.payload
}

// EncodeSource writes any source of CodecZenohFragment into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohFragmentSource, w codec.SceSink) error {
	if err := codec.WriteVLEU64(w, uint64(src.Sn())); err != nil {
		return err
	}
	if err := w.WriteBytes(src.Payload().Bytes()); err != nil {
		return err
	}
	return nil
}

// DecodeCodecZenohFragment decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohFragment(cursor *codec.SceCursor) (*CodecZenohFragment, error) {
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
	_bPayload, err := codec.MakeBytes[CodecZenohFragmentPayloadBound](Payload)
	if err != nil {
		return nil, err
	}
	return &CodecZenohFragment{
		sn: Sn,
		payload: _bPayload,
	}, nil
}

// Encode writes the CodecZenohFragment into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohFragment) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohFragment) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 65545)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
