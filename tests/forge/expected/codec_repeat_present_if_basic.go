// SCE-MAP: codec_repeat_present_if_basic:37 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_repeat_present_if_basic

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
	"example.com/sce-forge/codec_repeat_elem"
)

// CodecRepeatPresentIfBasicElemsBound is the bound of the elems element of CodecRepeatPresentIfBasic: 0 to 32 entries
// (docs/adr/0065 §6 R2). A List of it is made only within it.
type CodecRepeatPresentIfBasicElemsBound struct{}

// Least is the fewest entries the element holds.
func (CodecRepeatPresentIfBasicElemsBound) Least() int { return 0 }

// Most is the most entries the element holds.
func (CodecRepeatPresentIfBasicElemsBound) Most() int { return 32 }

// CodecRepeatPresentIfBasicSource is what the encode of CodecRepeatPresentIfBasic reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecRepeatPresentIfBasic is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecRepeatPresentIfBasicSource interface {
	Carrier() uint8
	Elems() optional.Of[codec.List[CodecRepeatPresentIfBasicElemsBound, codec_repeat_elem.CodecRepeatElemSource]]
}

// CodecRepeatPresentIfBasic represents the codec frame layout.
type CodecRepeatPresentIfBasic struct {
	carrier uint8
	elems optional.Of[codec.List[CodecRepeatPresentIfBasicElemsBound, codec_repeat_elem.CodecRepeatElemSource]]
}

// MakeCodecRepeatPresentIfBasic returns the CodecRepeatPresentIfBasic that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecRepeatPresentIfBasic, or as its zero value, and is read through its methods.
func MakeCodecRepeatPresentIfBasic(carrier uint8, elems optional.Of[codec.List[CodecRepeatPresentIfBasicElemsBound, codec_repeat_elem.CodecRepeatElemSource]]) CodecRepeatPresentIfBasic {
	return CodecRepeatPresentIfBasic{carrier: carrier, elems: elems}
}

// Carrier returns the element the value presents as Carrier.
func (s *CodecRepeatPresentIfBasic) Carrier() uint8 {
	return s.carrier
}

// Elems returns the element the value presents as Elems.
func (s *CodecRepeatPresentIfBasic) Elems() optional.Of[codec.List[CodecRepeatPresentIfBasicElemsBound, codec_repeat_elem.CodecRepeatElemSource]] {
	return s.elems
}

// WireNumElems returns the num_elems the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecRepeatPresentIfBasic) WireNumElems() optional.Of[uint8] {
	if _v, ok := s.Elems().Get(); ok {
		return optional.Some(uint8(_v.Len()))
	}
	return optional.None[uint8]()
}

// EncodeSource writes any source of CodecRepeatPresentIfBasic into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives num_elems from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecRepeatPresentIfBasicSource, w codec.SceSink) error {
	var _dNumElems uint8
	if src.Elems().IsPresent() {
		_dNumElems = uint8(src.Elems().Proven().Len())
	}
	_cCarrier := src.Carrier()
	_cCarrier &^= 0x1
	if src.Elems().IsPresent() {
		_cCarrier |= 0x1
	}
	if err := w.WriteBytes([]byte{byte(_cCarrier)}); err != nil {
		return err
	}
	if ((_cCarrier >> 0) & 1) != 0 {
		if err := w.WriteBytes([]byte{byte(_dNumElems)}); err != nil {
			return err
		}
	}
	if src.Elems().IsPresent() {
		for _e := range src.Elems().Proven().All() {
			if err := codec_repeat_elem.EncodeSource(_e, w); err != nil {
				return err
			}
		}
	}
	return nil
}

// DecodeCodecRepeatPresentIfBasic decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecRepeatPresentIfBasic(cursor *codec.SceCursor) (*CodecRepeatPresentIfBasic, error) {
	// Streaming cursor decode (SSOT selection: `needs_streaming`).
	// The positional `raw[byte_off]` path is valid only when every
	// field's absolute offset is fixed at codegen time; this branch
	// handles every codec where it is not — present-if-gated fields
	// (runtime presence, held by the runtime's optional type), VLE / repeat / TLV-chain /
	// embed fields (runtime width), string fields (UTF-8 decode), and a
	// fixed field after a variable-length payload (offset depends on the
	// payload length). Each field reads its own bytes from the cursor and
	// advances past what it consumed. Per-field `is_repeat` /
	// `is_tlv_chain` / `is_embed` route to their dedicated helpers; every
	// other field flows through `present_if_decode_stmt`.
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
	var NumElems optional.Of[uint8]
	if (Carrier & 0x01) != 0 {
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		_v := raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
		NumElems = optional.Some(_v)
	}
	var Elems optional.Of[[]codec_repeat_elem.CodecRepeatElem]
	if (Carrier & 0x01) != 0 {
		_n := NumElems.Proven()
		if _n > 32 {
			return nil, codec.ErrTooManyElements
		}
		_list := make([]codec_repeat_elem.CodecRepeatElem, 0, cursor.EntriesBacked(uint64(_n)))
		for _i := 0; _i < int(_n); _i++ {
			_elem, err := codec_repeat_elem.DecodeCodecRepeatElem(cursor)
			if err != nil {
				return nil, err
			}
			_list = append(_list, *_elem)
		}
		Elems = optional.Some(_list)
	}
	_lElems, err := codec.MakeOptionalList[CodecRepeatPresentIfBasicElemsBound](codec.OptionalSourcesOf(Elems, func(e *codec_repeat_elem.CodecRepeatElem) codec_repeat_elem.CodecRepeatElemSource { return e }))
	if err != nil {
		return nil, err
	}
	return &CodecRepeatPresentIfBasic{
		carrier: Carrier,
		elems: _lElems,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecRepeatPresentIfBasic) HasList() bool {
	return (s.carrier & 0x01) != 0
}

func (s *CodecRepeatPresentIfBasic) SetHasList(v bool) {
	if v {
		s.carrier |= 0x01
	} else {
		s.carrier &^= 0x01
	}
}

// Encode writes the CodecRepeatPresentIfBasic into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecRepeatPresentIfBasic) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
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
