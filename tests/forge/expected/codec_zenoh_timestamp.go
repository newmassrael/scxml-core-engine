// SCE-MAP: codec_zenoh_timestamp:34 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_timestamp

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohTimestampZidBound is the bound of the zid element of CodecZenohTimestamp: 0 to 16 bytes
// (docs/adr/0077 §3). A Bytes of it is made only within it.
type CodecZenohTimestampZidBound struct{}

// Least is the fewest bytes the element holds.
func (CodecZenohTimestampZidBound) Least() int { return 0 }

// Most is the most bytes the element holds.
func (CodecZenohTimestampZidBound) Most() int { return 16 }

// CodecZenohTimestampSource is what the encode of CodecZenohTimestamp reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohTimestamp is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohTimestampSource interface {
	Time() uint64
	Zid() codec.Bytes[CodecZenohTimestampZidBound]
}

// CodecZenohTimestamp represents the codec frame layout.
type CodecZenohTimestamp struct {
	time uint64
	zid codec.Bytes[CodecZenohTimestampZidBound]
}

// MakeCodecZenohTimestamp returns the CodecZenohTimestamp that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohTimestamp, or as its zero value, and is read through its methods.
func MakeCodecZenohTimestamp(time uint64, zid codec.Bytes[CodecZenohTimestampZidBound]) CodecZenohTimestamp {
	return CodecZenohTimestamp{time: time, zid: zid}
}

// Time returns the element the value presents as Time.
func (s *CodecZenohTimestamp) Time() uint64 {
	return s.time
}

// Zid returns the element the value presents as Zid.
func (s *CodecZenohTimestamp) Zid() codec.Bytes[CodecZenohTimestampZidBound] {
	return s.zid
}

// WireZidLen returns the zid_len the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecZenohTimestamp) WireZidLen() uint64 {
	return uint64(s.Zid().Len())
}

// EncodeSource writes any source of CodecZenohTimestamp into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives zid_len from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohTimestampSource, w codec.SceSink) error {
	_dZidLen := uint64(src.Zid().Len())
	if err := codec.WriteVLEU64(w, uint64(src.Time())); err != nil {
		return err
	}
	if err := codec.WriteVLEU64(w, uint64(_dZidLen)); err != nil {
		return err
	}
	if err := w.WriteBytes(src.Zid().Bytes()); err != nil {
		return err
	}
	return nil
}

// DecodeCodecZenohTimestamp decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohTimestamp(cursor *codec.SceCursor) (*CodecZenohTimestamp, error) {
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
	Time, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	ZidLen, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var Zid []byte
	{
		_len := ZidLen
		if _len > 16 {
			return nil, codec.ErrLengthOutOfRange
		}
		_n := int(_len)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Zid = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	_bZid, err := codec.MakeBytes[CodecZenohTimestampZidBound](Zid)
	if err != nil {
		return nil, err
	}
	return &CodecZenohTimestamp{
		time: Time,
		zid: _bZid,
	}, nil
}

// Encode writes the CodecZenohTimestamp into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohTimestamp) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohTimestamp) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 34)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
