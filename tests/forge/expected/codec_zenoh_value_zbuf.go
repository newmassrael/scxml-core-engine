// SCE-MAP: codec_zenoh_value_zbuf:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_value_zbuf

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_value_slice"
)

// CodecZenohValueZbuf represents the codec frame layout.
type CodecZenohValueZbuf struct {
	ValueLen uint64
	Value []byte
	Encoding *uint64
	SliceCount *uint64
	Slices []codec_zenoh_value_slice.CodecZenohValueSlice
}

// DecodeCodecZenohValueZbuf decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohValueZbuf(cursor *codec.SceCursor, afterShm byte) (*CodecZenohValueZbuf, error) {
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
	ValueLen, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var Value []byte
	if (afterShm & 0x01) == 0 {
		_n := int(ValueLen)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Value = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	var Encoding *uint64
	if (afterShm & 0x01) != 0 {
		_v, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
		Encoding = &_v
	}
	var SliceCount *uint64
	if (afterShm & 0x01) != 0 {
		_v, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
		SliceCount = &_v
	}
	var Slices []codec_zenoh_value_slice.CodecZenohValueSlice
	if (afterShm & 0x01) != 0 {
		_n := *SliceCount
		Slices = make([]codec_zenoh_value_slice.CodecZenohValueSlice, 0, _n)
		for _i := 0; _i < int(_n); _i++ {
			_elem, err := codec_zenoh_value_slice.DecodeCodecZenohValueSlice(cursor)
			if err != nil {
				return nil, err
			}
			Slices = append(Slices, *_elem)
		}
	}
	return &CodecZenohValueZbuf{
		ValueLen: ValueLen,
		Value: Value,
		Encoding: Encoding,
		SliceCount: SliceCount,
		Slices: Slices,
	}, nil
}

// Encode writes the CodecZenohValueZbuf into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohValueZbuf) Encode(w codec.SceSink, afterShm byte) error {
	// Streaming cursor encode (SSOT selection: `needs_streaming`).
	// Mirrors the streaming decode: every field appends its own bytes in
	// declaration order through the per-field encode blocks, so a gated
	// field skips its append when absent, and a fixed field after a
	// variable-length payload lands after the payload (the positional path
	// appends variable fields last, placing it ahead on the wire).
	// Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
	// dedicated helpers; everything else uses `present_if_encode_block`.
	if err := codec.WriteVLEU64(w, uint64(s.ValueLen)); err != nil {
		return err
	}
	if s.Value != nil {
		if err := w.WriteBytes(s.Value); err != nil {
			return err
		}
	}
	if s.Encoding != nil {
		_v := *s.Encoding
	if err := codec.WriteVLEU64(w, uint64(_v)); err != nil {
		return err
	}
	}
	if s.SliceCount != nil {
		_v := *s.SliceCount
	if err := codec.WriteVLEU64(w, uint64(_v)); err != nil {
		return err
	}
	}
	if s.Slices != nil {
		for _i := range s.Slices {
			if err := s.Slices[_i].Encode(w); err != nil {
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
func (s *CodecZenohValueZbuf) EncodeToBytes(afterShm byte) []byte {
	_dst := make([]byte, 0, 163)
	_ = s.Encode(codec.NewBytesSink(&_dst), afterShm)
	return _dst
}
