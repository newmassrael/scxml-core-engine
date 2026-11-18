// SCE-MAP: codec_nested_body:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_nested_body

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_locator"
)

// CodecNestedBodyLocsBound is the bound of the locs element of CodecNestedBody: 0 to 4 entries
// (docs/adr/0065 §6 R2). A List of it is made only within it.
type CodecNestedBodyLocsBound struct{}

// Least is the fewest entries the element holds.
func (CodecNestedBodyLocsBound) Least() int { return 0 }

// Most is the most entries the element holds.
func (CodecNestedBodyLocsBound) Most() int { return 4 }

// CodecNestedBodySource is what the encode of CodecNestedBody reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecNestedBody is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecNestedBodySource interface {
	Locs() codec.List[CodecNestedBodyLocsBound, codec_zenoh_locator.CodecZenohLocatorSource]
}

// CodecNestedBody represents the codec frame layout.
type CodecNestedBody struct {
	locs codec.List[CodecNestedBodyLocsBound, codec_zenoh_locator.CodecZenohLocatorSource]
}

// MakeCodecNestedBody returns the CodecNestedBody that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecNestedBody, or as its zero value, and is read through its methods.
func MakeCodecNestedBody(locs codec.List[CodecNestedBodyLocsBound, codec_zenoh_locator.CodecZenohLocatorSource]) CodecNestedBody {
	return CodecNestedBody{locs: locs}
}

// Locs returns the element the value presents as Locs.
func (s *CodecNestedBody) Locs() codec.List[CodecNestedBodyLocsBound, codec_zenoh_locator.CodecZenohLocatorSource] {
	return s.locs
}

// WireN returns the n the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecNestedBody) WireN() uint8 {
	return uint8(s.Locs().Len())
}

// EncodeSource writes any source of CodecNestedBody into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives n from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecNestedBodySource, w codec.SceSink) error {
	_dN := uint8(src.Locs().Len())
	if err := w.WriteBytes([]byte{byte(_dN)}); err != nil {
		return err
	}
	for _e := range src.Locs().All() {
		if err := codec_zenoh_locator.EncodeSource(_e, w); err != nil {
			return err
		}
	}
	return nil
}

// DecodeCodecNestedBody decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecNestedBody(cursor *codec.SceCursor) (*CodecNestedBody, error) {
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
	var N uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		N = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	if N > 4 {
		return nil, codec.ErrTooManyElements
	}
	Locs := make([]codec_zenoh_locator.CodecZenohLocator, 0, cursor.EntriesBacked(uint64(N)))
	for _i := 0; _i < int(N); _i++ {
		_elem, err := codec_zenoh_locator.DecodeCodecZenohLocator(cursor)
		if err != nil {
			return nil, err
		}
		Locs = append(Locs, *_elem)
	}
	_lLocs, err := codec.MakeList[CodecNestedBodyLocsBound](codec.SourcesOf(Locs, func(e *codec_zenoh_locator.CodecZenohLocator) codec_zenoh_locator.CodecZenohLocatorSource { return e }))
	if err != nil {
		return nil, err
	}
	return &CodecNestedBody{
		locs: _lLocs,
	}, nil
}

// Encode writes the CodecNestedBody into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecNestedBody) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecNestedBody) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 1057)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
