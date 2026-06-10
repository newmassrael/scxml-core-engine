// SCE-MAP: codec_zenoh_msg_del:53

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_msg_del

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_timestamp"
	"example.com/sce-forge/codec_zenoh_ext_entry"
)

// CodecZenohMsgDel represents the codec frame layout.
type CodecZenohMsgDel struct {
	Header uint8
	Timestamp *codec_zenoh_timestamp.CodecZenohTimestamp
	Extensions []codec_zenoh_ext_entry.CodecZenohExtEntry
}

// NewCodecZenohMsgDel returns a CodecZenohMsgDel initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecZenohMsgDel().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecZenohMsgDel{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecZenohMsgDel() *CodecZenohMsgDel {
	return &CodecZenohMsgDel{
		Header: uint8(0x02),
	}
}

// DecodeCodecZenohMsgDel decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohMsgDel(cursor *codec.SceCursor) (*CodecZenohMsgDel, error) {
	// RFC §5.B present-if primitive: streaming decode
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
	var Timestamp *codec_zenoh_timestamp.CodecZenohTimestamp
	if (Header & 0x20) != 0 {
		_emb, err := codec_zenoh_timestamp.DecodeCodecZenohTimestamp(cursor)
		if err != nil {
			return nil, err
		}
		Timestamp = _emb
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
	return &CodecZenohMsgDel{
		Header: Header,
		Timestamp: Timestamp,
		Extensions: Extensions,
	}, nil
}

// RFC §5.B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohMsgDel) Mid() uint8 {
	return uint8((s.Header >> 0) & 0x1F)
}

func (s *CodecZenohMsgDel) SetMid(v uint8) {
	const _shiftedMask uint8 = 0x1F << 0
	_val := (uint8(v) & 0x1F) << 0
	s.Header = (s.Header &^ _shiftedMask) | _val
}

func (s *CodecZenohMsgDel) T() bool {
	return (s.Header & 0x20) != 0
}

func (s *CodecZenohMsgDel) SetT(v bool) {
	if v {
		s.Header |= 0x20
	} else {
		s.Header &^= 0x20
	}
}

func (s *CodecZenohMsgDel) X() bool {
	return (s.Header & 0x40) != 0
}

func (s *CodecZenohMsgDel) SetX(v bool) {
	if v {
		s.Header |= 0x40
	} else {
		s.Header &^= 0x40
	}
}

func (s *CodecZenohMsgDel) Z() bool {
	return (s.Header & 0x80) != 0
}

func (s *CodecZenohMsgDel) SetZ(v bool) {
	if v {
		s.Header |= 0x80
	} else {
		s.Header &^= 0x80
	}
}

// Encode writes the CodecZenohMsgDel into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohMsgDel) Encode(w codec.SceSink) error {
	// RFC §5.B present-if encode.
	if err := w.WriteBytes([]byte{ s.Header }); err != nil {
		return err
	}
	if s.Timestamp != nil {
		if err := s.Timestamp.Encode(w); err != nil {
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
func (s *CodecZenohMsgDel) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 429)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
