// SCE-MAP: codec_fixed_after_lengthref:19

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_fixed_after_lengthref

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecFixedAfterLengthref represents the codec frame layout.
type CodecFixedAfterLengthref struct {
	Header uint8
	PayloadLen uint16
	Payload []byte
	Crc32 uint32
}

// DecodeCodecFixedAfterLengthref decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecFixedAfterLengthref(cursor *codec.SceCursor) (*CodecFixedAfterLengthref, error) {
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
	var Header uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Header = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var PayloadLen uint16
	{
		raw, err := cursor.PeekSlice(2)
		if err != nil {
			return nil, err
		}
		PayloadLen = uint16(raw[0]) | uint16(raw[1])<<8
		if err := cursor.Advance(2); err != nil {
			return nil, err
		}
	}
	var Payload []byte
	{
		_n := int(PayloadLen)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Payload = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	var Crc32 uint32
	{
		raw, err := cursor.PeekSlice(4)
		if err != nil {
			return nil, err
		}
		Crc32 = uint32(raw[0]) | uint32(raw[1])<<8 | uint32(raw[2])<<16 | uint32(raw[3])<<24
		if err := cursor.Advance(4); err != nil {
			return nil, err
		}
	}
	return &CodecFixedAfterLengthref{
		Header: Header,
		PayloadLen: PayloadLen,
		Payload: Payload,
		Crc32: Crc32,
	}, nil
}

// Encode writes the CodecFixedAfterLengthref into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecFixedAfterLengthref) Encode(w codec.SceSink) error {
	// Streaming cursor encode (SSOT selection: `needs_streaming`).
	// Mirrors the streaming decode: every field appends its own bytes in
	// declaration order through the per-field encode blocks, so a gated
	// field skips its append when absent, and a fixed field after a
	// variable-length payload lands after the payload (the positional path
	// appends variable fields last, placing it ahead on the wire).
	// Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
	// dedicated helpers; everything else uses `present_if_encode_block`.
	if err := w.WriteBytes([]byte{ s.Header }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ byte(s.PayloadLen) }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ byte(s.PayloadLen>>8) }); err != nil {
		return err
	}
	if err := w.WriteBytes(s.Payload); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ byte(s.Crc32) }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ byte(s.Crc32>>8) }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ byte(s.Crc32>>16) }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ byte(s.Crc32>>24) }); err != nil {
		return err
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecFixedAfterLengthref) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 1508)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
