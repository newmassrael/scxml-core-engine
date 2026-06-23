// SCE-MAP: codec_zenoh_decl_queryable:46

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_decl_queryable

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_wireexpr"
)

// CodecZenohDeclQueryable represents the codec frame layout.
type CodecZenohDeclQueryable struct {
	Id uint32
	Wireexpr codec_zenoh_wireexpr.CodecZenohWireexpr
	ExtType *uint8
	ExtValue *uint64
}

// DecodeCodecZenohDeclQueryable decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohDeclQueryable(cursor *codec.SceCursor, N byte, Z byte) (*CodecZenohDeclQueryable, error) {
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
	Id, err := cursor.ReadVLEU32()
	if err != nil { return nil, err }
	var Wireexpr codec_zenoh_wireexpr.CodecZenohWireexpr
	{
		_emb, err := codec_zenoh_wireexpr.DecodeCodecZenohWireexpr(cursor, N)
		if err != nil {
			return nil, err
		}
		Wireexpr = *_emb
	}
	var ExtType *uint8
	if (Z & 0x01) != 0 {
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		_v := raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
		ExtType = &_v
	}
	var ExtValue *uint64
	if (Z & 0x01) != 0 {
		_v, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
		ExtValue = &_v
	}
	return &CodecZenohDeclQueryable{
		Id: Id,
		Wireexpr: Wireexpr,
		ExtType: ExtType,
		ExtValue: ExtValue,
	}, nil
}

// Encode writes the CodecZenohDeclQueryable into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohDeclQueryable) Encode(w codec.SceSink, N byte, Z byte) error {
	// Streaming cursor encode (SSOT selection: `needs_streaming`).
	// Mirrors the streaming decode: every field appends its own bytes in
	// declaration order through the per-field encode blocks, so a gated
	// field skips its append when absent, and a fixed field after a
	// variable-length payload lands after the payload (the positional path
	// appends variable fields last, placing it ahead on the wire).
	// Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
	// dedicated helpers; everything else uses `present_if_encode_block`.
	{
		_vle := uint64(s.Id)
		_vn := 0
		for _vle >= 0x80 && _vn < 4 {
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
	if err := s.Wireexpr.Encode(w, N); err != nil {
		return err
	}
	if s.ExtType != nil {
		_v := *s.ExtType
		if err := w.WriteBytes([]byte{ _v }); err != nil {
			return err
		}
	}
	if s.ExtValue != nil {
		_v := *s.ExtValue
	{
		_vle := uint64(_v)
		_vn := 0
		for _vle >= 0x80 && _vn < 8 {
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
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohDeclQueryable) EncodeToBytes(N byte, Z byte) []byte {
	_dst := make([]byte, 0, 273)
	_ = s.Encode(codec.NewBytesSink(&_dst), N, Z)
	return _dst
}
