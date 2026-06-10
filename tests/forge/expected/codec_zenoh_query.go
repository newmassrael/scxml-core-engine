// SCE-MAP: codec_zenoh_query:51

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_query

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_ext_entry"
)

// CodecZenohQuery represents the codec frame layout.
type CodecZenohQuery struct {
	Header uint8
	Consolidation *uint8
	ParametersLen *uint64
	Parameters []byte
	Extensions []codec_zenoh_ext_entry.CodecZenohExtEntry
}

// NewCodecZenohQuery returns a CodecZenohQuery initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecZenohQuery().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecZenohQuery{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecZenohQuery() *CodecZenohQuery {
	return &CodecZenohQuery{
		Header: uint8(0x03),
	}
}

// DecodeCodecZenohQuery decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohQuery(cursor *codec.SceCursor) (*CodecZenohQuery, error) {
	// RFC §synth-5-B present-if primitive: streaming decode
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
	var Consolidation *uint8
	if (Header & 0x20) != 0 {
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		_v := raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
		Consolidation = &_v
	}
	var ParametersLen *uint64
	if (Header & 0x40) != 0 {
		_v, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
		ParametersLen = &_v
	}
	var Parameters []byte
	if (Header & 0x40) != 0 {
		_n := int(*ParametersLen)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Parameters = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	var Extensions []codec_zenoh_ext_entry.CodecZenohExtEntry
	if (Header & 0x80) != 0 {
		Extensions = make([]codec_zenoh_ext_entry.CodecZenohExtEntry, 0, 8)
		for _i := 0; _i < int(8); _i++ {
			if cursor.Remaining() == 0 {
				break
			}
			_elem, err := codec_zenoh_ext_entry.DecodeCodecZenohExtEntry(cursor)
			if err != nil {
				return nil, err
			}
			Extensions = append(Extensions, *_elem)
		}
		if cursor.Remaining() > 0 {
			return nil, codec.ErrTlvChainOverflow
		}
	}
	return &CodecZenohQuery{
		Header: Header,
		Consolidation: Consolidation,
		ParametersLen: ParametersLen,
		Parameters: Parameters,
		Extensions: Extensions,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohQuery) Mid() uint8 {
	return uint8((s.Header >> 0) & 0x1F)
}

func (s *CodecZenohQuery) SetMid(v uint8) {
	const _shiftedMask uint8 = 0x1F << 0
	_val := (uint8(v) & 0x1F) << 0
	s.Header = (s.Header &^ _shiftedMask) | _val
}

func (s *CodecZenohQuery) C() bool {
	return (s.Header & 0x20) != 0
}

func (s *CodecZenohQuery) SetC(v bool) {
	if v {
		s.Header |= 0x20
	} else {
		s.Header &^= 0x20
	}
}

func (s *CodecZenohQuery) P() bool {
	return (s.Header & 0x40) != 0
}

func (s *CodecZenohQuery) SetP(v bool) {
	if v {
		s.Header |= 0x40
	} else {
		s.Header &^= 0x40
	}
}

func (s *CodecZenohQuery) Z() bool {
	return (s.Header & 0x80) != 0
}

func (s *CodecZenohQuery) SetZ(v bool) {
	if v {
		s.Header |= 0x80
	} else {
		s.Header &^= 0x80
	}
}

// Encode writes the CodecZenohQuery into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohQuery) Encode(w codec.SceSink) error {
	// RFC §synth-5-B present-if encode.
	if err := w.WriteBytes([]byte{ s.Header }); err != nil {
		return err
	}
	if s.Consolidation != nil {
		_v := *s.Consolidation
		if err := w.WriteBytes([]byte{ _v }); err != nil {
			return err
		}
	}
	if s.ParametersLen != nil {
		_v := *s.ParametersLen
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
	if s.Parameters != nil {
		if err := w.WriteBytes(s.Parameters); err != nil {
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
func (s *CodecZenohQuery) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 612)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
