// SCE-MAP: codec_little_endian:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_little_endian

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecLittleEndianSource is what the encode of CodecLittleEndian reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecLittleEndian is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecLittleEndianSource interface {
	SensorId() uint8
	Value() uint16
	Status() uint8
}

// CodecLittleEndian represents the codec frame layout.
type CodecLittleEndian struct {
	sensorId uint8
	value uint16
	status uint8
}

// MakeCodecLittleEndian returns the CodecLittleEndian that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecLittleEndian, or as its zero value, and is read through its methods.
func MakeCodecLittleEndian(sensorId uint8, value uint16, status uint8) CodecLittleEndian {
	return CodecLittleEndian{sensorId: sensorId, value: value, status: status}
}

// SensorId returns the element the value presents as SensorId.
func (s *CodecLittleEndian) SensorId() uint8 {
	return s.sensorId
}

// Value returns the element the value presents as Value.
func (s *CodecLittleEndian) Value() uint16 {
	return s.value
}

// Status returns the element the value presents as Status.
func (s *CodecLittleEndian) Status() uint8 {
	return s.status
}

// EncodeSource writes any source of CodecLittleEndian into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecLittleEndianSource, w codec.SceSink) error {
	if err := w.WriteBytes([]byte{byte(src.SensorId())}); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{byte(src.Value() & 0xFF), byte(src.Value() >> 8 & 0xFF)}); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{byte(src.Status())}); err != nil {
		return err
	}
	return nil
}

// DecodeCodecLittleEndian decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
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
		sensorId: SensorId,
		value: Value,
		status: Status,
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
	return EncodeSource(s, w)
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
