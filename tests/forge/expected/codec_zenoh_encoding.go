// SCE-MAP: codec_zenoh_encoding:68

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_encoding

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"unicode/utf8"
)

// CodecZenohEncoding represents the codec frame layout.
type CodecZenohEncoding struct {
	PackedId uint32
	SchemaLen *uint64
	Schema *string
}

// DecodeCodecZenohEncoding decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohEncoding(cursor *codec.SceCursor) (*CodecZenohEncoding, error) {
	// RFC §5.B present-if primitive: streaming decode
	// advances the cursor per field. Gated fields use `*T` for fixed
	// (nil = absent) or `[]byte` (nil = absent) for tail/length-ref;
	// VLE gating uses `*T` like fixed. Per-field `is_repeat` routes
	// Repeat fields to the dedicated helper. Branch fires before
	// has_vle_fields so a codec mixing VLE + present-if uses the
	// unified streaming path.
	PackedId, err := cursor.ReadVLEU32()
	if err != nil { return nil, err }
	var SchemaLen *uint64
	if (PackedId & 0x00000001) != 0 {
		_v, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
		SchemaLen = &_v
	}
	var Schema *string
	if (PackedId & 0x00000001) != 0 {
		_n := int(*SchemaLen)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		if !utf8.Valid(raw) {
			return nil, codec.ErrInvalidUTF8
		}
		_v := string(raw)
		Schema = &_v
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	return &CodecZenohEncoding{
		PackedId: PackedId,
		SchemaLen: SchemaLen,
		Schema: Schema,
	}, nil
}

// RFC §5.B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohEncoding) HasSchema() bool {
	return (s.PackedId & 0x00000001) != 0
}

func (s *CodecZenohEncoding) SetHasSchema(v bool) {
	if v {
		s.PackedId |= 0x00000001
	} else {
		s.PackedId &^= 0x00000001
	}
}

// Encode writes the CodecZenohEncoding into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohEncoding) Encode(w codec.SceSink) error {
	// RFC §5.B present-if encode.
	{
		_vle := uint64(s.PackedId)
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
	if s.SchemaLen != nil {
		_v := *s.SchemaLen
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
	if s.Schema != nil {
		if err := w.WriteBytes([]byte(*s.Schema)); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohEncoding) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 143)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
