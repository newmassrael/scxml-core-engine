// SCE-MAP: codec_qos_byte:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_qos_byte

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
)

// CodecQosByteSource is what the encode of CodecQosByte reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecQosByte is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecQosByteSource interface {
	Qos() uint8
}

// CodecQosByte represents the codec frame layout.
type CodecQosByte struct {
	qos uint8
}

// MakeCodecQosByte returns the CodecQosByte that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecQosByte, or as its zero value, and is read through its methods.
func MakeCodecQosByte(qos uint8) CodecQosByte {
	return CodecQosByte{qos: qos}
}

// Qos returns the element the value presents as Qos.
func (s *CodecQosByte) Qos() uint8 {
	return s.qos
}

// CodecQosByteOverrides holds the one-bit flags of CodecQosByte a parent may
// set in place of the ones a value presents, such as the flag a chain whose
// entries are CodecQosByte ends them on, which the chain derives from where
// each entry stands. A flag it leaves absent is the one the value presents.
type CodecQosByteOverrides struct {
	// QosReliable is the `reliable` flag of `qos`, where a parent sets it.
	QosReliable optional.Of[bool]
	// QosExpress is the `express` flag of `qos`, where a parent sets it.
	QosExpress optional.Of[bool]
	// QosReserved is the `reserved` flag of `qos`, where a parent sets it.
	QosReserved optional.Of[bool]
}

// EncodeSource writes any source of CodecQosByte into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecQosByteSource, w codec.SceSink) error {
	return EncodeSourceOverriding(src, w, CodecQosByteOverrides{})
}

// EncodeSourceOverriding is EncodeSource, each flag overrides sets written in
// place of the one src presents.
func EncodeSourceOverriding(src CodecQosByteSource, w codec.SceSink, overrides CodecQosByteOverrides) error {
	_cQos := src.Qos()
	if _o, ok := overrides.QosReliable.Get(); ok {
		_cQos &^= 0x8
		if _o {
			_cQos |= 0x8
		}
	}
	if _o, ok := overrides.QosExpress.Get(); ok {
		_cQos &^= 0x40
		if _o {
			_cQos |= 0x40
		}
	}
	if _o, ok := overrides.QosReserved.Get(); ok {
		_cQos &^= 0x80
		if _o {
			_cQos |= 0x80
		}
	}
	if err := w.WriteBytes([]byte{byte(_cQos)}); err != nil {
		return err
	}
	return nil
}

// DecodeCodecQosByte decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecQosByte(cursor *codec.SceCursor) (*CodecQosByte, error) {
	raw, err := cursor.PeekSlice(1)
	if err != nil {
		return nil, err
	}
	Qos := raw[0]
	value := &CodecQosByte{
		qos: Qos,
	}
	if err := cursor.Advance(1); err != nil {
		return nil, err
	}
	return value, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecQosByte) Priority() uint8 {
	return uint8((s.qos >> 0) & 0x07)
}

func (s *CodecQosByte) SetPriority(v uint8) {
	const _shiftedMask uint8 = 0x07 << 0
	_val := (uint8(v) & 0x07) << 0
	s.qos = (s.qos &^ _shiftedMask) | _val
}

func (s *CodecQosByte) Reliable() bool {
	return (s.qos & 0x08) != 0
}

func (s *CodecQosByte) SetReliable(v bool) {
	if v {
		s.qos |= 0x08
	} else {
		s.qos &^= 0x08
	}
}

func (s *CodecQosByte) Congestion() uint8 {
	return uint8((s.qos >> 4) & 0x03)
}

func (s *CodecQosByte) SetCongestion(v uint8) {
	const _shiftedMask uint8 = 0x03 << 4
	_val := (uint8(v) & 0x03) << 4
	s.qos = (s.qos &^ _shiftedMask) | _val
}

func (s *CodecQosByte) Express() bool {
	return (s.qos & 0x40) != 0
}

func (s *CodecQosByte) SetExpress(v bool) {
	if v {
		s.qos |= 0x40
	} else {
		s.qos &^= 0x40
	}
}

func (s *CodecQosByte) Reserved() bool {
	return (s.qos & 0x80) != 0
}

func (s *CodecQosByte) SetReserved(v bool) {
	if v {
		s.qos |= 0x80
	} else {
		s.qos &^= 0x80
	}
}

// Encode writes the CodecQosByte into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecQosByte) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecQosByte) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 1)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
