// SCE-MAP: codec_embed_basic:43

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_embed_basic

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_locator"
)

// CodecEmbedBasic represents the codec frame layout.
type CodecEmbedBasic struct {
	Tag uint8
	Locator codec_zenoh_locator.CodecZenohLocator
}

// DecodeCodecEmbedBasic decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecEmbedBasic(cursor *codec.SceCursor) (*CodecEmbedBasic, error) {
	// RFC §synth-5-B B2 repeat primitive: streaming decode mixes plain
	// fixed-width reads (per-field via the present-if helper's
	// non-gated arm) with `make([]T, 0, N)` + `append` loops that
	// iterate the imported codec's `Decode<T>(cursor)` either
	// `int(N)` times (length-field) or until `cursor.Remaining() == 0`
	// (until-eof). Element bodies recurse into their own decoder —
	// each may itself surface `codec.ErrNeedMoreBytes`, unwinding
	// the partial frame.
	var Tag uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Tag = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var Locator codec_zenoh_locator.CodecZenohLocator
	{
		_emb, err := codec_zenoh_locator.DecodeCodecZenohLocator(cursor)
		if err != nil {
			return nil, err
		}
		Locator = *_emb
	}
	return &CodecEmbedBasic{
		Tag: Tag,
		Locator: Locator,
	}, nil
}

// Encode writes the CodecEmbedBasic into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecEmbedBasic) Encode(w codec.SceSink) error {
	// RFC §synth-5-B B2 encode: list fields range over s.<Pascal> and
	// write each element through the same sink.
	if err := w.WriteBytes([]byte{ s.Tag }); err != nil {
		return err
	}
	if err := s.Locator.Encode(w); err != nil {
		return err
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecEmbedBasic) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 257)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
