// SCE-MAP: codec_zenoh_locator:26 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_locator

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"unicode/utf8"
)

// CodecZenohLocatorLocatorBound is the bound of the locator element of CodecZenohLocator: 0 to 255 UTF-8 bytes
// (docs/adr/0077 §3). A Text of it is made only within it.
type CodecZenohLocatorLocatorBound struct{}

// Least is the fewest UTF-8 bytes the element holds.
func (CodecZenohLocatorLocatorBound) Least() int { return 0 }

// Most is the most UTF-8 bytes the element holds.
func (CodecZenohLocatorLocatorBound) Most() int { return 255 }

// CodecZenohLocatorSource is what the encode of CodecZenohLocator reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohLocator is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohLocatorSource interface {
	Locator() codec.Text[CodecZenohLocatorLocatorBound]
}

// CodecZenohLocator represents the codec frame layout.
type CodecZenohLocator struct {
	locator codec.Text[CodecZenohLocatorLocatorBound]
}

// MakeCodecZenohLocator returns the CodecZenohLocator that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohLocator, or as its zero value, and is read through its methods.
func MakeCodecZenohLocator(locator codec.Text[CodecZenohLocatorLocatorBound]) CodecZenohLocator {
	return CodecZenohLocator{locator: locator}
}

// Locator returns the element the value presents as Locator.
func (s *CodecZenohLocator) Locator() codec.Text[CodecZenohLocatorLocatorBound] {
	return s.locator
}

// WireLocatorLen returns the locator_len the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecZenohLocator) WireLocatorLen() uint64 {
	return uint64(s.Locator().Len())
}

// EncodeSource writes any source of CodecZenohLocator into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives locator_len from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohLocatorSource, w codec.SceSink) error {
	_dLocatorLen := uint64(src.Locator().Len())
	if err := codec.WriteVLEU64(w, uint64(_dLocatorLen)); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte(src.Locator().String())); err != nil {
		return err
	}
	return nil
}

// DecodeCodecZenohLocator decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohLocator(cursor *codec.SceCursor) (*CodecZenohLocator, error) {
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
	LocatorLen, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var Locator string
	{
		_len := LocatorLen
		if _len > 255 {
			return nil, codec.ErrLengthOutOfRange
		}
		_n := int(_len)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		if !utf8.Valid(raw) {
			return nil, codec.ErrInvalidUTF8
		}
		Locator = string(raw)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	_bLocator, err := codec.MakeText[CodecZenohLocatorLocatorBound](string(Locator))
	if err != nil {
		return nil, err
	}
	return &CodecZenohLocator{
		locator: _bLocator,
	}, nil
}

// Encode writes the CodecZenohLocator into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohLocator) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohLocator) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 264)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
