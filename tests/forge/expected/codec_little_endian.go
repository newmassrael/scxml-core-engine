// SCE-MAP: codec_little_endian:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_little_endian

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecLittleEndian represents the codec frame layout.
type CodecLittleEndian struct {
	SensorId uint8
	Value uint16
	Status uint8
}

// DecodeCodecLittleEndian decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecLittleEndian(cursor *codec.SceCursor) (*CodecLittleEndian, error) {
	raw, err := cursor.PeekSlice(4)
	if err != nil {
		return nil, err
	}
	SensorId := raw[0]
	Value := uint16(raw[1]) | uint16(raw[2])<<8
	Status := raw[3]
	value := &CodecLittleEndian{
		SensorId: SensorId,
		Value: Value,
		Status: Status,
	}
	if err := cursor.Advance(4); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode writes the CodecLittleEndian into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecLittleEndian) Encode(w codec.SceSink) error {
	if err := w.WriteBytes([]byte{ byte(s.SensorId) }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ byte(s.Value & 0xFF) }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ byte(s.Value >> 8 & 0xFF) }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ byte(s.Status) }); err != nil {
		return err
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecLittleEndian) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 4)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
