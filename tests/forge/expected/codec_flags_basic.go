// SCE-MAP: codec_flags_basic:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_flags_basic

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
)

// CodecFlagsBasicSource is what the encode of CodecFlagsBasic reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecFlagsBasic is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecFlagsBasicSource interface {
	Header() uint8
}

// CodecFlagsBasic represents the codec frame layout.
type CodecFlagsBasic struct {
	header uint8
}

// MakeCodecFlagsBasic returns the CodecFlagsBasic that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecFlagsBasic, or as its zero value, and is read through its methods.
func MakeCodecFlagsBasic(header uint8) CodecFlagsBasic {
	return CodecFlagsBasic{header: header}
}

// Header returns the element the value presents as Header.
func (s *CodecFlagsBasic) Header() uint8 {
	return s.header
}

// CodecFlagsBasicOverrides holds the one-bit flags of CodecFlagsBasic a parent may
// set in place of the ones a value presents, such as the flag a chain whose
// entries are CodecFlagsBasic ends them on, which the chain derives from where
// each entry stands. A flag it leaves absent is the one the value presents.
type CodecFlagsBasicOverrides struct {
	// HeaderReliable is the `reliable` flag of `header`, where a parent sets it.
	HeaderReliable optional.Of[bool]
	// HeaderMore is the `more` flag of `header`, where a parent sets it.
	HeaderMore optional.Of[bool]
	// HeaderDrop is the `drop` flag of `header`, where a parent sets it.
	HeaderDrop optional.Of[bool]
	// HeaderFirst is the `first` flag of `header`, where a parent sets it.
	HeaderFirst optional.Of[bool]
}

// EncodeSource writes any source of CodecFlagsBasic into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecFlagsBasicSource, w codec.SceSink) error {
	return EncodeSourceOverriding(src, w, CodecFlagsBasicOverrides{})
}

// EncodeSourceOverriding is EncodeSource, each flag overrides sets written in
// place of the one src presents.
func EncodeSourceOverriding(src CodecFlagsBasicSource, w codec.SceSink, overrides CodecFlagsBasicOverrides) error {
	_cHeader := src.Header()
	if _o, ok := overrides.HeaderReliable.Get(); ok {
		_cHeader &^= 0x80
		if _o {
			_cHeader |= 0x80
		}
	}
	if _o, ok := overrides.HeaderMore.Get(); ok {
		_cHeader &^= 0x40
		if _o {
			_cHeader |= 0x40
		}
	}
	if _o, ok := overrides.HeaderDrop.Get(); ok {
		_cHeader &^= 0x20
		if _o {
			_cHeader |= 0x20
		}
	}
	if _o, ok := overrides.HeaderFirst.Get(); ok {
		_cHeader &^= 0x10
		if _o {
			_cHeader |= 0x10
		}
	}
	if err := w.WriteBytes([]byte{byte(_cHeader)}); err != nil {
		return err
	}
	return nil
}

// DecodeCodecFlagsBasic decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecFlagsBasic(cursor *codec.SceCursor) (*CodecFlagsBasic, error) {
	raw, err := cursor.PeekSlice(1)
	if err != nil {
		return nil, err
	}
	Header := raw[0]
	value := &CodecFlagsBasic{
		header: Header,
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
func (s *CodecFlagsBasic) Reliable() bool {
	return (s.header & 0x80) != 0
}

func (s *CodecFlagsBasic) SetReliable(v bool) {
	if v {
		s.header |= 0x80
	} else {
		s.header &^= 0x80
	}
}

func (s *CodecFlagsBasic) More() bool {
	return (s.header & 0x40) != 0
}

func (s *CodecFlagsBasic) SetMore(v bool) {
	if v {
		s.header |= 0x40
	} else {
		s.header &^= 0x40
	}
}

func (s *CodecFlagsBasic) Drop() bool {
	return (s.header & 0x20) != 0
}

func (s *CodecFlagsBasic) SetDrop(v bool) {
	if v {
		s.header |= 0x20
	} else {
		s.header &^= 0x20
	}
}

func (s *CodecFlagsBasic) First() bool {
	return (s.header & 0x10) != 0
}

func (s *CodecFlagsBasic) SetFirst(v bool) {
	if v {
		s.header |= 0x10
	} else {
		s.header &^= 0x10
	}
}

// Encode writes the CodecFlagsBasic into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecFlagsBasic) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecFlagsBasic) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 1)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
