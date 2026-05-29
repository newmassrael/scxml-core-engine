// SCE-MAP: codec_nested_body:18

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_nested_body

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_locator"
)

// CodecNestedBody represents the codec frame layout.
type CodecNestedBody struct {
	N uint8
	Locs []codec_zenoh_locator.CodecZenohLocator
}

// DecodeCodecNestedBody decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecNestedBody(cursor *codec.SceCursor) (*CodecNestedBody, error) {
	// RFC §5.B B2 repeat primitive: streaming decode mixes plain
	// fixed-width reads (per-field via the present-if helper's
	// non-gated arm) with `make([]T, 0, N)` + `append` loops that
	// iterate the imported codec's `Decode<T>(cursor)` either
	// `int(N)` times (length-field) or until `cursor.Remaining() == 0`
	// (until-eof). Element bodies recurse into their own decoder —
	// each may itself surface `codec.ErrNeedMoreBytes`, unwinding
	// the partial frame.
	var N uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		N = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	Locs := make([]codec_zenoh_locator.CodecZenohLocator, 0, N)
	for _i := 0; _i < int(N); _i++ {
		_elem, err := codec_zenoh_locator.DecodeCodecZenohLocator(cursor)
		if err != nil {
			return nil, err
		}
		Locs = append(Locs, *_elem)
	}
	return &CodecNestedBody{
		N: N,
		Locs: Locs,
	}, nil
}

// Encode writes the CodecNestedBody into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecNestedBody) Encode(w codec.SceSink) error {
	// RFC §5.B B2 encode: list fields range over s.<Pascal> and
	// write each element through the same sink.
	if err := w.WriteBytes([]byte{ s.N }); err != nil {
		return err
	}
	for _i := range s.Locs {
		if err := s.Locs[_i].Encode(w); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecNestedBody) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 553)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
