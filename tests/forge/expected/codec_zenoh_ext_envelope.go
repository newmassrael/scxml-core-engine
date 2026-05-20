// SCE-MAP: codec_zenoh_ext_envelope:35

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_ext_envelope

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_ext_entry"
)

// CodecZenohExtEnvelope represents the codec frame layout.
type CodecZenohExtEnvelope struct {
	HeaderFlags uint8
	Extensions []codec_zenoh_ext_entry.CodecZenohExtEntry
}

// DecodeCodecZenohExtEnvelope decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohExtEnvelope(cursor *codec.SceCursor) (*CodecZenohExtEnvelope, error) {
	// RFC §5.B B2 repeat primitive: streaming decode mixes plain
	// fixed-width reads (per-field via the present-if helper's
	// non-gated arm) with `make([]T, 0, N)` + `append` loops that
	// iterate the imported codec's `Decode<T>(cursor)` either
	// `int(N)` times (length-field) or until `cursor.Remaining() == 0`
	// (until-eof). Element bodies recurse into their own decoder —
	// each may itself surface `codec.ErrNeedMoreBytes`, unwinding
	// the partial frame.
	var HeaderFlags uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		HeaderFlags = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	Extensions := make([]codec_zenoh_ext_entry.CodecZenohExtEntry, 0, 8)
	for _i := 0; _i < int(8); _i++ {
		if cursor.Remaining() == 0 {
			break
		}
		_elem, err := codec_zenoh_ext_entry.DecodeCodecZenohExtEntry(cursor)
		if err != nil {
			return nil, err
		}
		_continue := _elem.Z()
		Extensions = append(Extensions, *_elem)
		if !_continue {
			break
		}
	}
	return &CodecZenohExtEnvelope{
		HeaderFlags: HeaderFlags,
		Extensions: Extensions,
	}, nil
}

// Encode writes the CodecZenohExtEnvelope into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohExtEnvelope) Encode(w codec.SceSink) error {
	// RFC §5.B B2 encode: list fields range over s.<Pascal> and
	// write each element through the same sink.
	if err := w.WriteBytes([]byte{ s.HeaderFlags }); err != nil {
		return err
	}
	for _i := range s.Extensions {
		if err := s.Extensions[_i].Encode(w); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohExtEnvelope) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 345)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
