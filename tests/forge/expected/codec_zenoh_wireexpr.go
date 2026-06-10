// SCE-MAP: codec_zenoh_wireexpr:53

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_wireexpr

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"unicode/utf8"
)

// CodecZenohWireexpr represents the codec frame layout.
type CodecZenohWireexpr struct {
	Id uint64
	SuffixLen *uint64
	Suffix *string
}

// DecodeCodecZenohWireexpr decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohWireexpr(cursor *codec.SceCursor, N byte) (*CodecZenohWireexpr, error) {
	// RFC §5.B present-if primitive: streaming decode
	// advances the cursor per field. Gated fields use `*T` for fixed
	// (nil = absent) or `[]byte` (nil = absent) for tail/length-ref;
	// VLE gating uses `*T` like fixed. Per-field `is_repeat` routes
	// Repeat fields to the dedicated helper. Branch fires before
	// has_vle_fields so a codec mixing VLE + present-if uses the
	// unified streaming path.
	Id, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var SuffixLen *uint64
	if (N & 0x01) != 0 {
		_v, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
		SuffixLen = &_v
	}
	var Suffix *string
	if (N & 0x01) != 0 {
		_n := int(*SuffixLen)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		if !utf8.Valid(raw) {
			return nil, codec.ErrInvalidUTF8
		}
		_v := string(raw)
		Suffix = &_v
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	return &CodecZenohWireexpr{
		Id: Id,
		SuffixLen: SuffixLen,
		Suffix: Suffix,
	}, nil
}

// Encode writes the CodecZenohWireexpr into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohWireexpr) Encode(w codec.SceSink, N byte) error {
	// RFC §5.B present-if encode.
	{
		_vle := uint64(s.Id)
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
	if s.SuffixLen != nil {
		_v := *s.SuffixLen
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
	if s.Suffix != nil {
		if err := w.WriteBytes([]byte(*s.Suffix)); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohWireexpr) EncodeToBytes(N byte) []byte {
	_dst := make([]byte, 0, 148)
	_ = s.Encode(codec.NewBytesSink(&_dst), N)
	return _dst
}
