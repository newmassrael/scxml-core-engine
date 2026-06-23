// SCE-MAP: codec_zenoh_ext_entry:52

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_ext_entry

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_ext_unit"
	"example.com/sce-forge/codec_zenoh_ext_zint"
	"example.com/sce-forge/codec_zenoh_ext_zbuf"
)

// CodecZenohExtEntryDefault bundles the runtime
// tag value with the catch-all body so encode can round-trip the
// observed tag back onto the wire (RFC §synth-5-B variant primitive).
type CodecZenohExtEntryDefault struct {
	Tag uint8
	Body codec_zenoh_ext_unit.CodecZenohExtUnit
}

// CodecZenohExtEntryVariant is a discriminated-union body for the codec's
// tag-field suffix (RFC §synth-5-B variant primitive). Exactly one of
// the pointer fields is non-nil at a time; the active arm is the one
// that matches the current tag value.
type CodecZenohExtEntryVariant struct {
	CodecZenohExtUnit *codec_zenoh_ext_unit.CodecZenohExtUnit
	CodecZenohExtZint *codec_zenoh_ext_zint.CodecZenohExtZint
	CodecZenohExtZbuf *codec_zenoh_ext_zbuf.CodecZenohExtZbuf
	Default *CodecZenohExtEntryDefault
}

// CodecZenohExtEntry represents the codec frame layout.
type CodecZenohExtEntry struct {
	Header uint8
	Body CodecZenohExtEntryVariant
}

// NewCodecZenohExtEntry returns a CodecZenohExtEntry initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecZenohExtEntry().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecZenohExtEntry{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecZenohExtEntry() *CodecZenohExtEntry {
	return &CodecZenohExtEntry{
		Body: CodecZenohExtEntryVariant{
			CodecZenohExtUnit: &codec_zenoh_ext_unit.CodecZenohExtUnit{},
		},
	}
}

// DecodeCodecZenohExtEntry decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohExtEntry(cursor *codec.SceCursor) (*CodecZenohExtEntry, error) {
	// Decode fixed prefix (RFC §synth-5-B variant: fields before tag suffix).
	raw, err := cursor.PeekSlice(1)
	if err != nil {
		return nil, err
	}
	Header := raw[0]
	if err := cursor.Advance(1); err != nil {
		return nil, err
	}
	// Dispatch on the tag field; each arm decodes its body codec from
	// the cursor. The default arm (when declared) carries the runtime
	// tag value so encode can round-trip it back onto the wire.
	body := CodecZenohExtEntryVariant{}
	switch uint8((Header >> 5) & 0x03) {
	case 0:
		_arm, err := codec_zenoh_ext_unit.DecodeCodecZenohExtUnit(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohExtUnit = _arm
	case 1:
		_arm, err := codec_zenoh_ext_zint.DecodeCodecZenohExtZint(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohExtZint = _arm
	case 2:
		_arm, err := codec_zenoh_ext_zbuf.DecodeCodecZenohExtZbuf(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohExtZbuf = _arm
	default:
		_arm, err := codec_zenoh_ext_unit.DecodeCodecZenohExtUnit(cursor)
		if err != nil {
			return nil, err
		}
		body.Default = &CodecZenohExtEntryDefault{
			Tag: uint8((Header >> 5) & 0x03),
			Body: *_arm,
		}
	}
	return &CodecZenohExtEntry{
		Header: Header,
		Body: body,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohExtEntry) ExtId() uint8 {
	return uint8((s.Header >> 0) & 0x0F)
}

func (s *CodecZenohExtEntry) SetExtId(v uint8) {
	const _shiftedMask uint8 = 0x0F << 0
	_val := (uint8(v) & 0x0F) << 0
	s.Header = (s.Header &^ _shiftedMask) | _val
}

func (s *CodecZenohExtEntry) M() bool {
	return (s.Header & 0x10) != 0
}

func (s *CodecZenohExtEntry) SetM(v bool) {
	if v {
		s.Header |= 0x10
	} else {
		s.Header &^= 0x10
	}
}

func (s *CodecZenohExtEntry) Enc() uint8 {
	return uint8((s.Header >> 5) & 0x03)
}

func (s *CodecZenohExtEntry) SetEnc(v uint8) {
	const _shiftedMask uint8 = 0x03 << 5
	_val := (uint8(v) & 0x03) << 5
	s.Header = (s.Header &^ _shiftedMask) | _val
}

func (s *CodecZenohExtEntry) Z() bool {
	return (s.Header & 0x80) != 0
}

func (s *CodecZenohExtEntry) SetZ(v bool) {
	if v {
		s.Header |= 0x80
	} else {
		s.Header &^= 0x80
	}
}

// Encode writes the CodecZenohExtEntry into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohExtEntry) Encode(w codec.SceSink) error {
	// Encode fixed prefix (tag field bytes are part of the prefix).
	if err := w.WriteBytes([]byte{ byte(s.Header) }); err != nil {
		return err
	}
	// Append the active arm body's encoded bytes via the same sink.
	switch {
	case s.Body.CodecZenohExtUnit != nil:
		if err := s.Body.CodecZenohExtUnit.Encode(w); err != nil {
			return err
		}
	case s.Body.CodecZenohExtZint != nil:
		if err := s.Body.CodecZenohExtZint.Encode(w); err != nil {
			return err
		}
	case s.Body.CodecZenohExtZbuf != nil:
		if err := s.Body.CodecZenohExtZbuf.Encode(w); err != nil {
			return err
		}
	case s.Body.Default != nil:
		if err := s.Body.Default.Body.Encode(w); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohExtEntry) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 42)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
