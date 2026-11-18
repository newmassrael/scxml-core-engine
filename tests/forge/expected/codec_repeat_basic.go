// SCE-MAP: codec_repeat_basic:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_repeat_basic

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_repeat_elem"
)

// CodecRepeatBasicFragsBound is the bound of the frags element of CodecRepeatBasic: 0 to 32 entries
// (docs/adr/0065 §6 R2). A List of it is made only within it.
type CodecRepeatBasicFragsBound struct{}

// Least is the fewest entries the element holds.
func (CodecRepeatBasicFragsBound) Least() int { return 0 }

// Most is the most entries the element holds.
func (CodecRepeatBasicFragsBound) Most() int { return 32 }

// CodecRepeatBasicSource is what the encode of CodecRepeatBasic reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecRepeatBasic is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecRepeatBasicSource interface {
	Frags() codec.List[CodecRepeatBasicFragsBound, codec_repeat_elem.CodecRepeatElemSource]
}

// CodecRepeatBasic represents the codec frame layout.
type CodecRepeatBasic struct {
	frags codec.List[CodecRepeatBasicFragsBound, codec_repeat_elem.CodecRepeatElemSource]
}

// MakeCodecRepeatBasic returns the CodecRepeatBasic that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecRepeatBasic, or as its zero value, and is read through its methods.
func MakeCodecRepeatBasic(frags codec.List[CodecRepeatBasicFragsBound, codec_repeat_elem.CodecRepeatElemSource]) CodecRepeatBasic {
	return CodecRepeatBasic{frags: frags}
}

// Frags returns the element the value presents as Frags.
func (s *CodecRepeatBasic) Frags() codec.List[CodecRepeatBasicFragsBound, codec_repeat_elem.CodecRepeatElemSource] {
	return s.frags
}

// WireNumFrags returns the num_frags the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecRepeatBasic) WireNumFrags() uint8 {
	return uint8(s.Frags().Len())
}

// EncodeSource writes any source of CodecRepeatBasic into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives num_frags from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecRepeatBasicSource, w codec.SceSink) error {
	_dNumFrags := uint8(src.Frags().Len())
	if err := w.WriteBytes([]byte{byte(_dNumFrags)}); err != nil {
		return err
	}
	for _e := range src.Frags().All() {
		if err := codec_repeat_elem.EncodeSource(_e, w); err != nil {
			return err
		}
	}
	return nil
}

// DecodeCodecRepeatBasic decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecRepeatBasic(cursor *codec.SceCursor) (*CodecRepeatBasic, error) {
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
	var NumFrags uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		NumFrags = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	if NumFrags > 32 {
		return nil, codec.ErrTooManyElements
	}
	Frags := make([]codec_repeat_elem.CodecRepeatElem, 0, cursor.EntriesBacked(uint64(NumFrags)))
	for _i := 0; _i < int(NumFrags); _i++ {
		_elem, err := codec_repeat_elem.DecodeCodecRepeatElem(cursor)
		if err != nil {
			return nil, err
		}
		Frags = append(Frags, *_elem)
	}
	_lFrags, err := codec.MakeList[CodecRepeatBasicFragsBound](codec.SourcesOf(Frags, func(e *codec_repeat_elem.CodecRepeatElem) codec_repeat_elem.CodecRepeatElemSource { return e }))
	if err != nil {
		return nil, err
	}
	return &CodecRepeatBasic{
		frags: _lFrags,
	}, nil
}

// Encode writes the CodecRepeatBasic into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecRepeatBasic) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecRepeatBasic) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 65)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
