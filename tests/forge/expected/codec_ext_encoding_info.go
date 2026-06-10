// SCE-MAP: codec_ext_encoding_info:44

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_ext_encoding_info

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecExtEncodingInfo represents the codec frame layout.
type CodecExtEncodingInfo struct {
	CombinedId uint32
	SchemaSize uint8
	Schema []byte
}

// DecodeCodecExtEncodingInfo decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecExtEncodingInfo(cursor *codec.SceCursor) (*CodecExtEncodingInfo, error) {
	// RFC §5.B present-if primitive: streaming decode
	// advances the cursor per field. Gated fields use `*T` for fixed
	// (nil = absent) or `[]byte` (nil = absent) for tail/length-ref;
	// VLE gating uses `*T` like fixed. Per-field `is_repeat` routes
	// Repeat fields to the dedicated helper. Branch fires before
	// has_vle_fields so a codec mixing VLE + present-if uses the
	// unified streaming path.
	CombinedId, err := cursor.ReadVLEU32()
	if err != nil { return nil, err }
	var SchemaSize uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		SchemaSize = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var Schema []byte
	if (CombinedId & 0x00000001) != 0 {
		_n := int(SchemaSize)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Schema = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	return &CodecExtEncodingInfo{
		CombinedId: CombinedId,
		SchemaSize: SchemaSize,
		Schema: Schema,
	}, nil
}

// RFC §5.B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecExtEncodingInfo) HasSchema() bool {
	return (s.CombinedId & 0x00000001) != 0
}

func (s *CodecExtEncodingInfo) SetHasSchema(v bool) {
	if v {
		s.CombinedId |= 0x00000001
	} else {
		s.CombinedId &^= 0x00000001
	}
}

// Encode writes the CodecExtEncodingInfo into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecExtEncodingInfo) Encode(w codec.SceSink) error {
	// RFC §5.B present-if encode.
	{
		_vle := uint64(s.CombinedId)
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
	if err := w.WriteBytes([]byte{ s.SchemaSize }); err != nil {
		return err
	}
	if s.Schema != nil {
		if err := w.WriteBytes(s.Schema); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecExtEncodingInfo) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 71)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
