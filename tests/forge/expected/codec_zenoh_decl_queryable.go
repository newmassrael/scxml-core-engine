// SCE-MAP: codec_zenoh_decl_queryable:46

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_decl_queryable

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_wireexpr"
)

// CodecZenohDeclQueryable represents the codec frame layout.
type CodecZenohDeclQueryable struct {
	Id uint32
	Wireexpr codec_zenoh_wireexpr.CodecZenohWireexpr
	ExtType *uint8
	ExtValue *uint64
}

// DecodeCodecZenohDeclQueryable decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohDeclQueryable(cursor *codec.SceCursor, N byte, Z byte) (*CodecZenohDeclQueryable, error) {
	// RFC §synth-5-B present-if primitive: streaming decode
	// advances the cursor per field. Gated fields use `*T` for fixed
	// (nil = absent) or `[]byte` (nil = absent) for tail/length-ref;
	// VLE gating uses `*T` like fixed. Per-field `is_repeat` routes
	// Repeat fields to the dedicated helper. Branch fires before
	// has_vle_fields so a codec mixing VLE + present-if uses the
	// unified streaming path.
	Id, err := cursor.ReadVLEU32()
	if err != nil { return nil, err }
	var Wireexpr codec_zenoh_wireexpr.CodecZenohWireexpr
	{
		_emb, err := codec_zenoh_wireexpr.DecodeCodecZenohWireexpr(cursor, N)
		if err != nil {
			return nil, err
		}
		Wireexpr = *_emb
	}
	var ExtType *uint8
	if (Z & 0x01) != 0 {
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		_v := raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
		ExtType = &_v
	}
	var ExtValue *uint64
	if (Z & 0x01) != 0 {
		_v, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
		ExtValue = &_v
	}
	return &CodecZenohDeclQueryable{
		Id: Id,
		Wireexpr: Wireexpr,
		ExtType: ExtType,
		ExtValue: ExtValue,
	}, nil
}

// Encode writes the CodecZenohDeclQueryable into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohDeclQueryable) Encode(w codec.SceSink, N byte, Z byte) error {
	// RFC §synth-5-B present-if encode.
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
	if err := s.Wireexpr.Encode(w, N); err != nil {
		return err
	}
	if s.ExtType != nil {
		_v := *s.ExtType
		if err := w.WriteBytes([]byte{ _v }); err != nil {
			return err
		}
	}
	if s.ExtValue != nil {
		_v := *s.ExtValue
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
func (s *CodecZenohDeclQueryable) EncodeToBytes(N byte, Z byte) []byte {
	_dst := make([]byte, 0, 274)
	_ = s.Encode(codec.NewBytesSink(&_dst), N, Z)
	return _dst
}
