// SCE-MAP: codec_zenoh_ext_zbuf:17

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_ext_zbuf

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohExtZbuf represents the codec frame layout.
type CodecZenohExtZbuf struct {
	ValueLen uint64
	Value []byte
}

// DecodeCodecZenohExtZbuf decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohExtZbuf(cursor *codec.SceCursor) (*CodecZenohExtZbuf, error) {
	// Streaming codec: each field reads from cursor directly
	// (VLE base-128 chain). Local var name reuses the Go-PascalCase
	// `field.id` — the struct literal's `Foo: Foo` is unambiguous
	// because the package owns both names. RFC §synth-5-B B4: per-field
	// bit-size dispatch routes Fixed / LengthRef siblings of VLE
	// fields through `present_if_decode_stmt` (predicate=None arms).
	// Pure-VLE codecs stay byte-stable.
	ValueLen, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var Value []byte
	{
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
	return &CodecZenohExtZbuf{
		ValueLen: ValueLen,
		Value: Value,
	}, nil
}

// Encode writes the CodecZenohExtZbuf into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohExtZbuf) Encode(w codec.SceSink) error {
	// RFC §synth-5-B B4: per-field bit-size dispatch.
	{
		_vle := uint64(s.ValueLen)
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
	if err := w.WriteBytes(s.Value); err != nil {
		return err
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohExtZbuf) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 42)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
