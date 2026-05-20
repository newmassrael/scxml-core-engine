// SCE-MAP: codec_zenoh_interest:73

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_interest

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_ext_entry"
	"example.com/sce-forge/codec_zenoh_interest_body"
)

// CodecZenohInterest represents the codec frame layout.
type CodecZenohInterest struct {
	Header uint8
	Id uint64
	Body *codec_zenoh_interest_body.CodecZenohInterestBody
	Extensions []codec_zenoh_ext_entry.CodecZenohExtEntry
}

// NewCodecZenohInterest returns a CodecZenohInterest initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecZenohInterest().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecZenohInterest{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity Atomic β-go.
func NewCodecZenohInterest() *CodecZenohInterest {
	return &CodecZenohInterest{
		Header: uint8(0x19),
	}
}

// DecodeCodecZenohInterest decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohInterest(cursor *codec.SceCursor) (*CodecZenohInterest, error) {
	// RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
	// advances the cursor per field. Gated fields use `*T` for fixed
	// (nil = absent) or `[]byte` (nil = absent) for tail/length-ref;
	// VLE gating uses `*T` like fixed. Per-field `is_repeat` routes
	// Repeat fields to the dedicated helper. Branch fires before
	// has_vle_fields so a codec mixing VLE + present-if uses the
	// unified streaming path.
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
	Id, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var Body *codec_zenoh_interest_body.CodecZenohInterestBody
	if (Header & 0x20) != 0 || (Header & 0x40) != 0 {
		_emb, err := codec_zenoh_interest_body.DecodeCodecZenohInterestBody(cursor)
		if err != nil {
			return nil, err
		}
		Body = _emb
	}
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
	return &CodecZenohInterest{
		Header: Header,
		Id: Id,
		Body: Body,
		Extensions: Extensions,
	}, nil
}

// RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohInterest) Mid() uint8 {
	return uint8((s.Header >> 0) & 0x1F)
}

func (s *CodecZenohInterest) SetMid(v uint8) {
	const _shiftedMask uint8 = 0x1F << 0
	_val := (uint8(v) & 0x1F) << 0
	s.Header = (s.Header &^ _shiftedMask) | _val
}

func (s *CodecZenohInterest) CURRENT() bool {
	return (s.Header & 0x20) != 0
}

func (s *CodecZenohInterest) SetCURRENT(v bool) {
	if v {
		s.Header |= 0x20
	} else {
		s.Header &^= 0x20
	}
}

func (s *CodecZenohInterest) FUTURE() bool {
	return (s.Header & 0x40) != 0
}

func (s *CodecZenohInterest) SetFUTURE(v bool) {
	if v {
		s.Header |= 0x40
	} else {
		s.Header &^= 0x40
	}
}

func (s *CodecZenohInterest) Z() bool {
	return (s.Header & 0x80) != 0
}

func (s *CodecZenohInterest) SetZ(v bool) {
	if v {
		s.Header |= 0x80
	} else {
		s.Header &^= 0x80
	}
}

// Encode writes the CodecZenohInterest into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohInterest) Encode(w codec.SceSink) error {
	// RFC §5.B B1-δ + B2-β present-if encode.
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
	if s.Body != nil {
		if err := s.Body.Encode(w); err != nil {
			return err
		}
	}
	for _i := range s.Extensions {
		if err := s.Extensions[_i].Encode(w); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohInterest) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 439)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
