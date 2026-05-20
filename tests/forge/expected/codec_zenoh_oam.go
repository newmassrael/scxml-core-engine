// SCE-MAP: codec_zenoh_oam:56

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_oam

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_ext_entry"
	"example.com/sce-forge/codec_zenoh_ext_unit"
	"example.com/sce-forge/codec_zenoh_ext_zint"
	"example.com/sce-forge/codec_zenoh_ext_zbuf"
)

// CodecZenohOamDefault bundles the runtime
// tag value with the catch-all body so encode can round-trip the
// observed tag back onto the wire (RFC §5.B variant primitive B1-β).
type CodecZenohOamDefault struct {
	Tag uint8
	Body codec_zenoh_ext_unit.CodecZenohExtUnit
}

// CodecZenohOamVariant is a discriminated-union body for the codec's
// tag-field suffix (RFC §5.B variant primitive B1-β). Exactly one of
// the pointer fields is non-nil at a time; the active arm is the one
// that matches the current tag value.
type CodecZenohOamVariant struct {
	CodecZenohExtUnit *codec_zenoh_ext_unit.CodecZenohExtUnit
	CodecZenohExtZint *codec_zenoh_ext_zint.CodecZenohExtZint
	CodecZenohExtZbuf *codec_zenoh_ext_zbuf.CodecZenohExtZbuf
	Default *CodecZenohOamDefault
}

// CodecZenohOam represents the codec frame layout.
type CodecZenohOam struct {
	Header uint8
	Id uint16
	Extensions []codec_zenoh_ext_entry.CodecZenohExtEntry
	Body CodecZenohOamVariant
}

// NewCodecZenohOam returns a CodecZenohOam initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecZenohOam().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecZenohOam{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity Atomic β-go.
func NewCodecZenohOam() *CodecZenohOam {
	return &CodecZenohOam{
		Header: uint8(0x1f),
		Body: CodecZenohOamVariant{
			CodecZenohExtUnit: &codec_zenoh_ext_unit.CodecZenohExtUnit{},
		},
	}
}

// DecodeCodecZenohOam decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohOam(cursor *codec.SceCursor) (*CodecZenohOam, error) {
	// RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
	// streaming prefix decode (variable-length fields supported via
	// per-field present_if/tlv-chain/embed/repeat helpers). Peek-byte
	// mode additionally peeks the cursor's next byte for variant tag
	// without advancing — arm body decoder reads it as own header.
	var Header uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Header = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	Id, err := cursor.ReadVLEU16()
	if err != nil { return nil, err }
	var Extensions []codec_zenoh_ext_entry.CodecZenohExtEntry
	if (Header & 0x80) != 0 {
		Extensions = make([]codec_zenoh_ext_entry.CodecZenohExtEntry, 0, 4)
		for _i := 0; _i < int(4); _i++ {
			if cursor.Remaining() == 0 {
				break
			}
			_elem, err := codec_zenoh_ext_entry.DecodeCodecZenohExtEntry(cursor)
			if err != nil {
				return nil, err
			}
			_continue := _elem.Z()
			Extensions = append(Extensions, *_elem)
			if !_continue {
				break
			}
		}
	}
	// Dispatch on the tag field; each arm decodes its body codec from
	// the cursor. The default arm (when declared) carries the runtime
	// tag value so encode can round-trip it back onto the wire.
	body := CodecZenohOamVariant{}
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
		body.Default = &CodecZenohOamDefault{
			Tag: uint8((Header >> 5) & 0x03),
			Body: *_arm,
		}
	}
	return &CodecZenohOam{
		Header: Header,
		Id: Id,
		Extensions: Extensions,
		Body: body,
	}, nil
}

// RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohOam) Mid() uint8 {
	return uint8((s.Header >> 0) & 0x1F)
}

func (s *CodecZenohOam) SetMid(v uint8) {
	const _shiftedMask uint8 = 0x1F << 0
	_val := (uint8(v) & 0x1F) << 0
	s.Header = (s.Header &^ _shiftedMask) | _val
}

func (s *CodecZenohOam) Enc() uint8 {
	return uint8((s.Header >> 5) & 0x03)
}

func (s *CodecZenohOam) SetEnc(v uint8) {
	const _shiftedMask uint8 = 0x03 << 5
	_val := (uint8(v) & 0x03) << 5
	s.Header = (s.Header &^ _shiftedMask) | _val
}

func (s *CodecZenohOam) Z() bool {
	return (s.Header & 0x80) != 0
}

func (s *CodecZenohOam) SetZ(v bool) {
	if v {
		s.Header |= 0x80
	} else {
		s.Header &^= 0x80
	}
}

// Encode writes the CodecZenohOam into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohOam) Encode(w codec.SceSink) error {
	// RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix.
	if err := w.WriteBytes([]byte{ s.Header }); err != nil {
		return err
	}
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
	for _i := range s.Extensions {
		if err := s.Extensions[_i].Encode(w); err != nil {
			return err
		}
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
func (s *CodecZenohOam) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 46)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
