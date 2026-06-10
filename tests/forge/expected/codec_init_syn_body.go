// SCE-MAP: codec_init_syn_body:30

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_init_syn_body

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecInitSynBody represents the codec frame layout.
type CodecInitSynBody struct {
	Version uint8
	SnRes *uint8
	BatchSize *uint16
}

// DecodeCodecInitSynBody decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecInitSynBody(cursor *codec.SceCursor, S byte) (*CodecInitSynBody, error) {
	// RFC §synth-5-B present-if primitive: streaming decode
	// advances the cursor per field. Gated fields use `*T` for fixed
	// (nil = absent) or `[]byte` (nil = absent) for tail/length-ref;
	// VLE gating uses `*T` like fixed. Per-field `is_repeat` routes
	// Repeat fields to the dedicated helper. Branch fires before
	// has_vle_fields so a codec mixing VLE + present-if uses the
	// unified streaming path.
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
	var SnRes *uint8
	if (S & 0x01) != 0 {
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		_v := raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
		SnRes = &_v
	}
	var BatchSize *uint16
	if (S & 0x01) != 0 {
		raw, err := cursor.PeekSlice(2)
		if err != nil {
			return nil, err
		}
		_v := uint16(raw[0])<<8 | uint16(raw[1])
		if err := cursor.Advance(2); err != nil {
			return nil, err
		}
		BatchSize = &_v
	}
	return &CodecInitSynBody{
		Version: Version,
		SnRes: SnRes,
		BatchSize: BatchSize,
	}, nil
}

// Encode writes the CodecInitSynBody into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecInitSynBody) Encode(w codec.SceSink, S byte) error {
	// RFC §synth-5-B present-if encode.
	if err := w.WriteBytes([]byte{ s.Version }); err != nil {
		return err
	}
	if s.SnRes != nil {
		_v := *s.SnRes
		if err := w.WriteBytes([]byte{ _v }); err != nil {
			return err
		}
	}
	if s.BatchSize != nil {
		_v := *s.BatchSize
		if err := w.WriteBytes([]byte{ byte(_v>>8) }); err != nil {
			return err
		}
		if err := w.WriteBytes([]byte{ byte(_v) }); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecInitSynBody) EncodeToBytes(S byte) []byte {
	_dst := make([]byte, 0, 4)
	_ = s.Encode(codec.NewBytesSink(&_dst), S)
	return _dst
}
