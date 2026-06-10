// SCE-MAP: codec_scout_zid_body:35

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_scout_zid_body

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecScoutZidBody represents the codec frame layout.
type CodecScoutZidBody struct {
	ZidLenM1 uint8
	Zid []byte
}

// DecodeCodecScoutZidBody decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecScoutZidBody(cursor *codec.SceCursor) (*CodecScoutZidBody, error) {
	frameLen := cursor.Remaining()
	if frameLen < 1 {
		return nil, codec.ErrNeedMoreBytes
	}
	raw, err := cursor.PeekSlice(frameLen)
	if err != nil {
		return nil, err
	}
	ZidLenM1 := raw[0]
	Zid := raw[1:1+int(ZidLenM1) + 1]
	value := &CodecScoutZidBody{
		ZidLenM1: ZidLenM1,
		Zid: Zid,
	}
	if err := cursor.Advance(frameLen); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode writes the CodecScoutZidBody into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecScoutZidBody) Encode(w codec.SceSink) error {
	if err := w.WriteBytes([]byte{ byte(s.ZidLenM1) }); err != nil {
		return err
	}
	if err := w.WriteBytes(s.Zid); err != nil {
		return err
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecScoutZidBody) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 17)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
