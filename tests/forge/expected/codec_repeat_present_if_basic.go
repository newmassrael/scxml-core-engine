// SCE-MAP: codec_repeat_present_if_basic:37

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_repeat_present_if_basic

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_repeat_elem"
)

// CodecRepeatPresentIfBasic represents the codec frame layout.
type CodecRepeatPresentIfBasic struct {
	Carrier uint8
	NumElems *uint8
	Elems []codec_repeat_elem.CodecRepeatElem
}

// DecodeCodecRepeatPresentIfBasic decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecRepeatPresentIfBasic(cursor *codec.SceCursor) (*CodecRepeatPresentIfBasic, error) {
	// RFC §synth-5-B present-if primitive: streaming decode
	// advances the cursor per field. Gated fields use `*T` for fixed
	// (nil = absent) or `[]byte` (nil = absent) for tail/length-ref;
	// VLE gating uses `*T` like fixed. Per-field `is_repeat` routes
	// Repeat fields to the dedicated helper. Branch fires before
	// has_vle_fields so a codec mixing VLE + present-if uses the
	// unified streaming path.
	var Carrier uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Carrier = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var NumElems *uint8
	if (Carrier & 0x01) != 0 {
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		_v := raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
		NumElems = &_v
	}
	var Elems []codec_repeat_elem.CodecRepeatElem
	if (Carrier & 0x01) != 0 {
		_n := *NumElems
		Elems = make([]codec_repeat_elem.CodecRepeatElem, 0, _n)
		for _i := 0; _i < int(_n); _i++ {
			_elem, err := codec_repeat_elem.DecodeCodecRepeatElem(cursor)
			if err != nil {
				return nil, err
			}
			Elems = append(Elems, *_elem)
		}
	}
	return &CodecRepeatPresentIfBasic{
		Carrier: Carrier,
		NumElems: NumElems,
		Elems: Elems,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecRepeatPresentIfBasic) HasList() bool {
	return (s.Carrier & 0x01) != 0
}

func (s *CodecRepeatPresentIfBasic) SetHasList(v bool) {
	if v {
		s.Carrier |= 0x01
	} else {
		s.Carrier &^= 0x01
	}
}

// Encode writes the CodecRepeatPresentIfBasic into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecRepeatPresentIfBasic) Encode(w codec.SceSink) error {
	// RFC §synth-5-B present-if encode.
	if err := w.WriteBytes([]byte{ s.Carrier }); err != nil {
		return err
	}
	if s.NumElems != nil {
		_v := *s.NumElems
		if err := w.WriteBytes([]byte{ _v }); err != nil {
			return err
		}
	}
	if s.Elems != nil {
		for _i := range s.Elems {
			if err := s.Elems[_i].Encode(w); err != nil {
				return err
			}
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecRepeatPresentIfBasic) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 66)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
