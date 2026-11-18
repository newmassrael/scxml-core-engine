// SCE-MAP: codec_zenoh_undecl_kexpr:35 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_undecl_kexpr

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohUndeclKexprSource is what the encode of CodecZenohUndeclKexpr reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohUndeclKexpr is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohUndeclKexprSource interface {
	Id() uint16
}

// CodecZenohUndeclKexpr represents the codec frame layout.
type CodecZenohUndeclKexpr struct {
	id uint16
}

// MakeCodecZenohUndeclKexpr returns the CodecZenohUndeclKexpr that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohUndeclKexpr, or as its zero value, and is read through its methods.
func MakeCodecZenohUndeclKexpr(id uint16) CodecZenohUndeclKexpr {
	return CodecZenohUndeclKexpr{id: id}
}

// Id returns the element the value presents as Id.
func (s *CodecZenohUndeclKexpr) Id() uint16 {
	return s.id
}

// EncodeSource writes any source of CodecZenohUndeclKexpr into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohUndeclKexprSource, w codec.SceSink) error {
	if err := codec.WriteVLEU16(w, uint16(src.Id())); err != nil {
		return err
	}
	return nil
}

// DecodeCodecZenohUndeclKexpr decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohUndeclKexpr(cursor *codec.SceCursor) (*CodecZenohUndeclKexpr, error) {
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
	Id, err := cursor.ReadVLEU16()
	if err != nil { return nil, err }
	return &CodecZenohUndeclKexpr{
		id: Id,
	}, nil
}

// Encode writes the CodecZenohUndeclKexpr into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohUndeclKexpr) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohUndeclKexpr) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 3)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
