// SCE-MAP: codec_ext_encoding_info:44 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_ext_encoding_info

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecExtEncodingInfo represents the codec frame layout.
type CodecExtEncodingInfo struct {
	CombinedId uint32
	SchemaSize uint8
	Schema []byte
}

// DecodeCodecExtEncodingInfo decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecExtEncodingInfo(cursor *codec.SceCursor) (*CodecExtEncodingInfo, error) {
	// Streaming cursor decode (SSOT selection: `needs_streaming`).
	// The positional `raw[byte_off]` path is valid only when every
	// field's absolute offset is fixed at codegen time; this branch
	// handles every codec where it is not — present-if-gated fields
	// (runtime presence; `*T` / nil `[]byte`), VLE / repeat / TLV-chain /
	// embed fields (runtime width), string fields (UTF-8 decode), and a
	// fixed field after a variable-length payload (offset depends on the
	// payload length). Each field reads its own bytes from the cursor and
	// advances past what it consumed. Per-field `is_repeat` /
	// `is_tlv_chain` / `is_embed` route to their dedicated helpers; every
	// other field flows through `present_if_decode_stmt`.
	CombinedId, err := cursor.ReadVLEU32()
	if err != nil { return nil, err }
	var SchemaSize uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		SchemaSize = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var Schema []byte
	if (CombinedId & 0x00000001) != 0 {
		_n := int(SchemaSize)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Schema = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	return &CodecExtEncodingInfo{
		CombinedId: CombinedId,
		SchemaSize: SchemaSize,
		Schema: Schema,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecExtEncodingInfo) HasSchema() bool {
	return (s.CombinedId & 0x00000001) != 0
}

func (s *CodecExtEncodingInfo) SetHasSchema(v bool) {
	if v {
		s.CombinedId |= 0x00000001
	} else {
		s.CombinedId &^= 0x00000001
	}
}

// Encode writes the CodecExtEncodingInfo into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecExtEncodingInfo) Encode(w codec.SceSink) error {
	// Streaming cursor encode (SSOT selection: `needs_streaming`).
	// Mirrors the streaming decode: every field appends its own bytes in
	// declaration order through the per-field encode blocks, so a gated
	// field skips its append when absent, and a fixed field after a
	// variable-length payload lands after the payload (the positional path
	// appends variable fields last, placing it ahead on the wire).
	// Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
	// dedicated helpers; everything else uses `present_if_encode_block`.
	if err := codec.WriteVLEU32(w, uint32(s.CombinedId)); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ s.SchemaSize }); err != nil {
		return err
	}
	if s.Schema != nil {
		if err := w.WriteBytes(s.Schema); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecExtEncodingInfo) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 71)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
