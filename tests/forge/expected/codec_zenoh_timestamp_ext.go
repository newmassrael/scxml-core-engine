// SCE-MAP: codec_zenoh_timestamp_ext:48 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_timestamp_ext

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_timestamp"
)

// CodecZenohTimestampExtSource is what the encode of CodecZenohTimestampExt reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohTimestampExt is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohTimestampExtSource interface {
	Ts() codec_zenoh_timestamp.CodecZenohTimestampSource
}

// CodecZenohTimestampExt represents the codec frame layout.
type CodecZenohTimestampExt struct {
	ts codec_zenoh_timestamp.CodecZenohTimestampSource
}

// MakeCodecZenohTimestampExt returns the CodecZenohTimestampExt that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohTimestampExt, or as its zero value, and is read through its methods.
func MakeCodecZenohTimestampExt(ts codec_zenoh_timestamp.CodecZenohTimestampSource) CodecZenohTimestampExt {
	return CodecZenohTimestampExt{ts: ts}
}

// Ts returns the element the value presents as Ts.
func (s *CodecZenohTimestampExt) Ts() codec_zenoh_timestamp.CodecZenohTimestampSource {
	if s.ts == nil {
		return &codec_zenoh_timestamp.CodecZenohTimestamp{}
	}
	return s.ts
}

// WireExtSize returns the ext_size the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecZenohTimestampExt) WireExtSize() uint64 {
	var _k codec.CountingSink
	_ = codec_zenoh_timestamp.EncodeSource(s.Ts(), &_k)
	return uint64(_k.Position())
}

// EncodeSource writes any source of CodecZenohTimestampExt into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives ext_size from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohTimestampExtSource, w codec.SceSink) error {
	var _kTs codec.CountingSink
	if err := codec_zenoh_timestamp.EncodeSource(src.Ts(), &_kTs); err != nil {
		return err
	}
	_dExtSize := uint64(_kTs.Position())
	if err := codec.WriteVLEU64(w, uint64(_dExtSize)); err != nil {
		return err
	}
	if err := codec_zenoh_timestamp.EncodeSource(src.Ts(), w); err != nil {
		return err
	}
	return nil
}

// DecodeCodecZenohTimestampExt decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohTimestampExt(cursor *codec.SceCursor) (*CodecZenohTimestampExt, error) {
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
	ExtSize, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var Ts codec_zenoh_timestamp.CodecZenohTimestamp
	{
		_len := int(ExtSize)
		_raw, err := cursor.PeekSlice(_len)
		if err != nil {
			return nil, err
		}
		_inner := codec.NewSceCursor(_raw)
		_emb, err := codec_zenoh_timestamp.DecodeCodecZenohTimestamp(&_inner)
		if err != nil {
			return nil, err
		}
		if err := cursor.Advance(_len); err != nil {
			return nil, err
		}
		Ts = *_emb
	}
	return &CodecZenohTimestampExt{
		ts: &Ts,
	}, nil
}

// Encode writes the CodecZenohTimestampExt into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohTimestampExt) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohTimestampExt) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 43)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
