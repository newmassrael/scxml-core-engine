// SCE-MAP: codec_zenoh_timestamp:34

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_timestamp

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohTimestamp represents the codec frame layout.
type CodecZenohTimestamp struct {
	Time uint64
	ZidLen uint64
	Zid []byte
}

// DecodeCodecZenohTimestamp decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohTimestamp(cursor *codec.SceCursor) (*CodecZenohTimestamp, error) {
	// Streaming codec: each field reads from cursor directly
	// (VLE base-128 chain). Local var name reuses the Go-PascalCase
	// `field.id` — the struct literal's `Foo: Foo` is unambiguous
	// because the package owns both names. RFC §5.B B4: per-field
	// bit-size dispatch routes Fixed / LengthRef siblings of VLE
	// fields through `present_if_decode_stmt` (predicate=None arms).
	// Pure-VLE codecs stay byte-stable.
	Time, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	ZidLen, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var Zid []byte
	{
		_n := int(ZidLen)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Zid = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	return &CodecZenohTimestamp{
		Time: Time,
		ZidLen: ZidLen,
		Zid: Zid,
	}, nil
}

// Encode writes the CodecZenohTimestamp into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohTimestamp) Encode(w codec.SceSink) error {
	// RFC §5.B B4: per-field bit-size dispatch.
	{
		_vle := uint64(s.Time)
		for _vle >= 0x80 {
			if err := w.WriteBytes([]byte{ byte(_vle&0x7F) | 0x80 }); err != nil {
				return err
			}
			_vle >>= 7
		}
		if err := w.WriteBytes([]byte{ byte(_vle) }); err != nil {
			return err
		}
	}
	{
		_vle := uint64(s.ZidLen)
		for _vle >= 0x80 {
			if err := w.WriteBytes([]byte{ byte(_vle&0x7F) | 0x80 }); err != nil {
				return err
			}
			_vle >>= 7
		}
		if err := w.WriteBytes([]byte{ byte(_vle) }); err != nil {
			return err
		}
	}
	if err := w.WriteBytes(s.Zid); err != nil {
		return err
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohTimestamp) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 36)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
