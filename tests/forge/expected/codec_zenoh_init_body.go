// SCE-MAP: codec_zenoh_init_body:42 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_init_body

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
)

// CodecZenohInitBody represents the codec frame layout.
type CodecZenohInitBody struct {
	Version uint8
	Cbyte uint8
	Zid []byte
	SnRes optional.Of[uint8]
	BatchSize optional.Of[uint16]
	CookieLen optional.Of[uint64]
	Cookie optional.Of[[]byte]
}

// DecodeCodecZenohInitBody decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohInitBody(cursor *codec.SceCursor, S byte, A byte) (*CodecZenohInitBody, error) {
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
	var Cbyte uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Cbyte = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var Zid []byte
	{
		_n := (int((Cbyte >> 4) & 0xF) + 1)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Zid = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	var SnRes optional.Of[uint8]
	if (S & 0x01) != 0 {
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		_v := raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
		SnRes = optional.Some(_v)
	}
	var BatchSize optional.Of[uint16]
	if (S & 0x01) != 0 {
		raw, err := cursor.PeekSlice(2)
		if err != nil {
			return nil, err
		}
		_v := uint16(raw[0]) | uint16(raw[1])<<8
		if err := cursor.Advance(2); err != nil {
			return nil, err
		}
		BatchSize = optional.Some(_v)
	}
	var CookieLen optional.Of[uint64]
	if (A & 0x01) != 0 {
		_v, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
		CookieLen = optional.Some(_v)
	}
	var Cookie optional.Of[[]byte]
	if (A & 0x01) != 0 {
		_n := int(CookieLen.Proven())
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Cookie = optional.Some(append([]byte(nil), raw...))
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	return &CodecZenohInitBody{
		Version: Version,
		Cbyte: Cbyte,
		Zid: Zid,
		SnRes: SnRes,
		BatchSize: BatchSize,
		CookieLen: CookieLen,
		Cookie: Cookie,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohInitBody) Whatami() uint8 {
	return uint8((s.Cbyte >> 0) & 0x03)
}

func (s *CodecZenohInitBody) SetWhatami(v uint8) {
	const _shiftedMask uint8 = 0x03 << 0
	_val := (uint8(v) & 0x03) << 0
	s.Cbyte = (s.Cbyte &^ _shiftedMask) | _val
}

func (s *CodecZenohInitBody) ZidLenM1() uint8 {
	return uint8((s.Cbyte >> 4) & 0x0F)
}

func (s *CodecZenohInitBody) SetZidLenM1(v uint8) {
	const _shiftedMask uint8 = 0x0F << 4
	_val := (uint8(v) & 0x0F) << 4
	s.Cbyte = (s.Cbyte &^ _shiftedMask) | _val
}

// Encode writes the CodecZenohInitBody into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohInitBody) Encode(w codec.SceSink, S byte, A byte) error {
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
	if err := w.WriteBytes([]byte{ s.Cbyte }); err != nil {
		return err
	}
	if err := w.WriteBytes(s.Zid); err != nil {
		return err
	}
	if s.SnRes.IsPresent() {
		_v := s.SnRes.Proven()
		if err := w.WriteBytes([]byte{ _v }); err != nil {
			return err
		}
	}
	if s.BatchSize.IsPresent() {
		_v := s.BatchSize.Proven()
		if err := w.WriteBytes([]byte{ byte(_v & 0xFF) }); err != nil {
			return err
		}
		if err := w.WriteBytes([]byte{ byte(_v >> 8 & 0xFF) }); err != nil {
			return err
		}
	}
	if s.CookieLen.IsPresent() {
		_v := s.CookieLen.Proven()
	if err := codec.WriteVLEU64(w, uint64(_v)); err != nil {
		return err
	}
	}
	if s.Cookie.IsPresent() {
		if err := w.WriteBytes(s.Cookie.Proven()); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohInitBody) EncodeToBytes(S byte, A byte) []byte {
	_dst := make([]byte, 0, 159)
	_ = s.Encode(codec.NewBytesSink(&_dst), S, A)
	return _dst
}
