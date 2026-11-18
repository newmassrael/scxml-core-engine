// SCE-MAP: codec_vle_zint_u64:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_vle_zint_u64

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecVleZintU64Source is what the encode of CodecVleZintU64 reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecVleZintU64 is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecVleZintU64Source interface {
	Value() uint64
}

// CodecVleZintU64 represents the codec frame layout.
type CodecVleZintU64 struct {
	value uint64
}

// MakeCodecVleZintU64 returns the CodecVleZintU64 that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecVleZintU64, or as its zero value, and is read through its methods.
func MakeCodecVleZintU64(value uint64) CodecVleZintU64 {
	return CodecVleZintU64{value: value}
}

// Value returns the element the value presents as Value.
func (s *CodecVleZintU64) Value() uint64 {
	return s.value
}

// EncodeSource writes any source of CodecVleZintU64 into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecVleZintU64Source, w codec.SceSink) error {
	if err := codec.WriteVLEU64(w, uint64(src.Value())); err != nil {
		return err
	}
	return nil
}

// DecodeCodecVleZintU64 decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecVleZintU64(cursor *codec.SceCursor) (*CodecVleZintU64, error) {
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
	Value, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	return &CodecVleZintU64{
		value: Value,
	}, nil
}

// Encode writes the CodecVleZintU64 into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecVleZintU64) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecVleZintU64) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 9)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
