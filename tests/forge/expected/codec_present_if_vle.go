// SCE-MAP: codec_present_if_vle:7

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_present_if_vle

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecPresentIfVle represents the codec frame layout.
type CodecPresentIfVle struct {
	Flags uint8
	OptionalId *uint64
}

// DecodeCodecPresentIfVle decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecPresentIfVle(cursor *codec.SceCursor) (*CodecPresentIfVle, error) {
	// RFC §5.B present-if primitive: streaming decode
	// advances the cursor per field. Gated fields use `*T` for fixed
	// (nil = absent) or `[]byte` (nil = absent) for tail/length-ref;
	// VLE gating uses `*T` like fixed. Per-field `is_repeat` routes
	// Repeat fields to the dedicated helper. Branch fires before
	// has_vle_fields so a codec mixing VLE + present-if uses the
	// unified streaming path.
	var Flags uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Flags = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var OptionalId *uint64
	if (Flags & 0x01) != 0 {
		_v, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
		OptionalId = &_v
	}
	return &CodecPresentIfVle{
		Flags: Flags,
		OptionalId: OptionalId,
	}, nil
}

// RFC §5.B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecPresentIfVle) HasId() bool {
	return (s.Flags & 0x01) != 0
}

func (s *CodecPresentIfVle) SetHasId(v bool) {
	if v {
		s.Flags |= 0x01
	} else {
		s.Flags &^= 0x01
	}
}

// Encode writes the CodecPresentIfVle into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecPresentIfVle) Encode(w codec.SceSink) error {
	// RFC §5.B present-if encode.
	if err := w.WriteBytes([]byte{ s.Flags }); err != nil {
		return err
	}
	if s.OptionalId != nil {
		_v := *s.OptionalId
	{
		_vle := uint64(_v)
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
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecPresentIfVle) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 11)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
