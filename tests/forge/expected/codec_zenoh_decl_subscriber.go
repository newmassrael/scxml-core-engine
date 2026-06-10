// SCE-MAP: codec_zenoh_decl_subscriber:41

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_decl_subscriber

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_wireexpr"
)

// CodecZenohDeclSubscriber represents the codec frame layout.
type CodecZenohDeclSubscriber struct {
	Id uint32
	Wireexpr codec_zenoh_wireexpr.CodecZenohWireexpr
}

// DecodeCodecZenohDeclSubscriber decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohDeclSubscriber(cursor *codec.SceCursor, N byte) (*CodecZenohDeclSubscriber, error) {
	// Streaming codec: each field reads from cursor directly
	// (VLE base-128 chain). Local var name reuses the Go-PascalCase
	// `field.id` — the struct literal's `Foo: Foo` is unambiguous
	// because the package owns both names. RFC §synth-5-B B4: per-field
	// bit-size dispatch routes Fixed / LengthRef siblings of VLE
	// fields through `present_if_decode_stmt` (predicate=None arms).
	// Pure-VLE codecs stay byte-stable.
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
	return &CodecZenohDeclSubscriber{
		Id: Id,
		Wireexpr: Wireexpr,
	}, nil
}

// Encode writes the CodecZenohDeclSubscriber into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohDeclSubscriber) Encode(w codec.SceSink, N byte) error {
	// RFC §synth-5-B B4: per-field bit-size dispatch.
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
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohDeclSubscriber) EncodeToBytes(N byte) []byte {
	_dst := make([]byte, 0, 261)
	_ = s.Encode(codec.NewBytesSink(&_dst), N)
	return _dst
}
