// SCE-MAP: codec_init_cookie_body:36

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_init_cookie_body

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecInitCookieBody represents the codec frame layout.
type CodecInitCookieBody struct {
	Version uint8
	CookieSize *uint16
	Cookie []byte
}

// DecodeCodecInitCookieBody decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecInitCookieBody(cursor *codec.SceCursor, A byte) (*CodecInitCookieBody, error) {
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
	var Version uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Version = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var CookieSize *uint16
	if (A & 0x01) != 0 {
		_v, err := cursor.ReadVLEU16()
	if err != nil { return nil, err }
		CookieSize = &_v
	}
	var Cookie []byte
	if (A & 0x01) != 0 {
		_n := int(*CookieSize)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Cookie = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	return &CodecInitCookieBody{
		Version: Version,
		CookieSize: CookieSize,
		Cookie: Cookie,
	}, nil
}

// Encode writes the CodecInitCookieBody into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecInitCookieBody) Encode(w codec.SceSink, A byte) error {
	// Streaming cursor encode (SSOT selection: `needs_streaming`).
	// Mirrors the streaming decode: every field appends its own bytes in
	// declaration order through the per-field encode blocks, so a gated
	// field skips its append when absent, and a fixed field after a
	// variable-length payload lands after the payload (the positional path
	// appends variable fields last, placing it ahead on the wire).
	// Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
	// dedicated helpers; everything else uses `present_if_encode_block`.
	if err := w.WriteBytes([]byte{ s.Version }); err != nil {
		return err
	}
	if s.CookieSize != nil {
		_v := *s.CookieSize
	{
		_vle := uint64(_v)
		_vn := 0
		for _vle >= 0x80 && _vn < 2 {
			if err := w.WriteBytes([]byte{ byte(_vle&0x7F) | 0x80 }); err != nil {
				return err
			}
			_vle >>= 7
			_vn++
		}
		if err := w.WriteBytes([]byte{ byte(_vle) }); err != nil {
			return err
		}
	}
	}
	if s.Cookie != nil {
		if err := w.WriteBytes(s.Cookie); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecInitCookieBody) EncodeToBytes(A byte) []byte {
	_dst := make([]byte, 0, 68)
	_ = s.Encode(codec.NewBytesSink(&_dst), A)
	return _dst
}
