// SCE-MAP: codec_zenoh_undecl_token:16

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_undecl_token

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_decl_ext_keyexpr"
)

// CodecZenohUndeclToken represents the codec frame layout.
type CodecZenohUndeclToken struct {
	Id uint32
	ExtKeyexpr *codec_zenoh_decl_ext_keyexpr.CodecZenohDeclExtKeyexpr
}

// DecodeCodecZenohUndeclToken decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohUndeclToken(cursor *codec.SceCursor, Z byte) (*CodecZenohUndeclToken, error) {
	// RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
	// advances the cursor per field. Gated fields use `*T` for fixed
	// (nil = absent) or `[]byte` (nil = absent) for tail/length-ref;
	// VLE gating uses `*T` like fixed. Per-field `is_repeat` routes
	// Repeat fields to the dedicated helper. Branch fires before
	// has_vle_fields so a codec mixing VLE + present-if uses the
	// unified streaming path.
	Id, err := cursor.ReadVLEU32()
	if err != nil { return nil, err }
	var ExtKeyexpr *codec_zenoh_decl_ext_keyexpr.CodecZenohDeclExtKeyexpr
	if (Z & 0x01) != 0 {
		_emb, err := codec_zenoh_decl_ext_keyexpr.DecodeCodecZenohDeclExtKeyexpr(cursor)
		if err != nil {
			return nil, err
		}
		ExtKeyexpr = _emb
	}
	return &CodecZenohUndeclToken{
		Id: Id,
		ExtKeyexpr: ExtKeyexpr,
	}, nil
}

// Encode writes the CodecZenohUndeclToken into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohUndeclToken) Encode(w codec.SceSink, Z byte) error {
	// RFC §5.B B1-δ + B2-β present-if encode.
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
	if s.ExtKeyexpr != nil {
		if err := s.ExtKeyexpr.Encode(w); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohUndeclToken) EncodeToBytes(Z byte) []byte {
	_dst := make([]byte, 0, 261)
	_ = s.Encode(codec.NewBytesSink(&_dst), Z)
	return _dst
}
