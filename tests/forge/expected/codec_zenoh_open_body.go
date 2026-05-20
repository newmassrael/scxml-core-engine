// SCE-MAP: codec_zenoh_open_body:41

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_open_body

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohOpenBody represents the codec frame layout.
type CodecZenohOpenBody struct {
	Lease uint64
	InitialSn uint64
	CookieLen *uint64
	Cookie []byte
}

// DecodeCodecZenohOpenBody decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohOpenBody(cursor *codec.SceCursor, A byte) (*CodecZenohOpenBody, error) {
	// RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
	// advances the cursor per field. Gated fields use `*T` for fixed
	// (nil = absent) or `[]byte` (nil = absent) for tail/length-ref;
	// VLE gating uses `*T` like fixed. Per-field `is_repeat` routes
	// Repeat fields to the dedicated helper. Branch fires before
	// has_vle_fields so a codec mixing VLE + present-if uses the
	// unified streaming path.
	Lease, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	InitialSn, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var CookieLen *uint64
	if (A & 0x01) == 0 {
		_v, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
		CookieLen = &_v
	}
	var Cookie []byte
	if (A & 0x01) == 0 {
		_n := int(*CookieLen)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Cookie = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	return &CodecZenohOpenBody{
		Lease: Lease,
		InitialSn: InitialSn,
		CookieLen: CookieLen,
		Cookie: Cookie,
	}, nil
}

// Encode writes the CodecZenohOpenBody into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohOpenBody) Encode(w codec.SceSink, A byte) error {
	// RFC §5.B B1-δ + B2-β present-if encode.
	{
		_vle := uint64(s.Lease)
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
	{
		_vle := uint64(s.InitialSn)
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
	if s.CookieLen != nil {
		_v := *s.CookieLen
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
	if s.Cookie != nil {
		if err := w.WriteBytes(s.Cookie); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohOpenBody) EncodeToBytes(A byte) []byte {
	_dst := make([]byte, 0, 158)
	_ = s.Encode(codec.NewBytesSink(&_dst), A)
	return _dst
}
