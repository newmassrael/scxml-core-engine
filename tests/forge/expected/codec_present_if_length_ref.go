// SCE-MAP: codec_present_if_length_ref:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_present_if_length_ref

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
)

// CodecPresentIfLengthRefPayloadBound is the bound of the payload element of CodecPresentIfLengthRef: 0 to 32 bytes
// (docs/adr/0077 §3). A Bytes of it is made only within it.
type CodecPresentIfLengthRefPayloadBound struct{}

// Least is the fewest bytes the element holds.
func (CodecPresentIfLengthRefPayloadBound) Least() int { return 0 }

// Most is the most bytes the element holds.
func (CodecPresentIfLengthRefPayloadBound) Most() int { return 32 }

// CodecPresentIfLengthRefFlagsHasPayloadHasPayload is the FlagsHasPayload group of CodecPresentIfLengthRef where `flags.has_payload` holds (docs/adr/0065 §6 R4): the
// members present then.
type CodecPresentIfLengthRefFlagsHasPayloadHasPayload struct {
	payload codec.Bytes[CodecPresentIfLengthRefPayloadBound]
}

// MakeCodecPresentIfLengthRefFlagsHasPayloadHasPayload returns the arm that holds each member it is handed.
func MakeCodecPresentIfLengthRefFlagsHasPayloadHasPayload(payload codec.Bytes[CodecPresentIfLengthRefPayloadBound]) CodecPresentIfLengthRefFlagsHasPayloadHasPayload {
	return CodecPresentIfLengthRefFlagsHasPayloadHasPayload{payload: payload}
}

// Payload returns the member the arm presents as Payload.
func (a CodecPresentIfLengthRefFlagsHasPayloadHasPayload) Payload() codec.Bytes[CodecPresentIfLengthRefPayloadBound] {
	return a.payload
}

func (CodecPresentIfLengthRefFlagsHasPayloadHasPayload) isCodecPresentIfLengthRefFlagsHasPayload() {}

// CodecPresentIfLengthRefFlagsHasPayloadNotHasPayload is the FlagsHasPayload group of CodecPresentIfLengthRef where `flags.has_payload` does not hold (docs/adr/0065 §6 R4): the
// members present then.
type CodecPresentIfLengthRefFlagsHasPayloadNotHasPayload struct {
	payloadSize uint8
}

// MakeCodecPresentIfLengthRefFlagsHasPayloadNotHasPayload returns the arm that holds each member it is handed.
func MakeCodecPresentIfLengthRefFlagsHasPayloadNotHasPayload(payloadSize uint8) CodecPresentIfLengthRefFlagsHasPayloadNotHasPayload {
	return CodecPresentIfLengthRefFlagsHasPayloadNotHasPayload{payloadSize: payloadSize}
}

// PayloadSize returns the member the arm presents as PayloadSize.
func (a CodecPresentIfLengthRefFlagsHasPayloadNotHasPayload) PayloadSize() uint8 {
	return a.payloadSize
}

func (CodecPresentIfLengthRefFlagsHasPayloadNotHasPayload) isCodecPresentIfLengthRefFlagsHasPayload() {}

// CodecPresentIfLengthRefFlagsHasPayload is the FlagsHasPayload group of CodecPresentIfLengthRef: the arm the value holds, each the
// members present while the gates hold its values (docs/adr/0065 §6 R4),
// one of CodecPresentIfLengthRefFlagsHasPayloadHasPayload, CodecPresentIfLengthRefFlagsHasPayloadNotHasPayload, which only this package declares.
type CodecPresentIfLengthRefFlagsHasPayload interface {
	isCodecPresentIfLengthRefFlagsHasPayload()
}

// CodecPresentIfLengthRefSource is what the encode of CodecPresentIfLengthRef reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecPresentIfLengthRef is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecPresentIfLengthRefSource interface {
	Flags() uint8
	FlagsHasPayload() CodecPresentIfLengthRefFlagsHasPayload
}

// CodecPresentIfLengthRef represents the codec frame layout.
type CodecPresentIfLengthRef struct {
	flags uint8
	flagsHasPayload CodecPresentIfLengthRefFlagsHasPayload
}

// MakeCodecPresentIfLengthRef returns the CodecPresentIfLengthRef that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecPresentIfLengthRef, or as its zero value, and is read through its methods.
func MakeCodecPresentIfLengthRef(flags uint8, flagsHasPayload CodecPresentIfLengthRefFlagsHasPayload) CodecPresentIfLengthRef {
	return CodecPresentIfLengthRef{flags: flags, flagsHasPayload: flagsHasPayload}
}

// Flags returns the element the value presents as Flags.
func (s *CodecPresentIfLengthRef) Flags() uint8 {
	return s.flags
}

// FlagsHasPayload returns the element the value presents as FlagsHasPayload.
func (s *CodecPresentIfLengthRef) FlagsHasPayload() CodecPresentIfLengthRefFlagsHasPayload {
	if s.flagsHasPayload == nil {
		return CodecPresentIfLengthRefFlagsHasPayloadHasPayload{}
	}
	return s.flagsHasPayload
}

// WirePayloadSize returns the payload_size the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecPresentIfLengthRef) WirePayloadSize() uint8 {
	_rFlagsHasPayload := s.FlagsHasPayload()
	var _mPayload optional.Of[codec.Bytes[CodecPresentIfLengthRefPayloadBound]]
	var _mPayloadSize optional.Of[uint8]
	switch _g := _rFlagsHasPayload.(type) {
	case CodecPresentIfLengthRefFlagsHasPayloadHasPayload:
		_mPayload = optional.Some(_g.Payload())
	case CodecPresentIfLengthRefFlagsHasPayloadNotHasPayload:
		_mPayloadSize = optional.Some(_g.PayloadSize())
	}
	if _v, ok := _mPayload.Get(); ok {
		return uint8(_v.Len())
	}
	if _s, ok := _mPayloadSize.Get(); ok {
		return _s
	}
	return 0
}

// EncodeSource writes any source of CodecPresentIfLengthRef into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives payload_size from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecPresentIfLengthRefSource, w codec.SceSink) error {
	_rFlagsHasPayload := src.FlagsHasPayload()
	var _mPayload optional.Of[codec.Bytes[CodecPresentIfLengthRefPayloadBound]]
	var _mPayloadSize optional.Of[uint8]
	switch _g := _rFlagsHasPayload.(type) {
	case CodecPresentIfLengthRefFlagsHasPayloadHasPayload:
		_mPayload = optional.Some(_g.Payload())
	case CodecPresentIfLengthRefFlagsHasPayloadNotHasPayload:
		_mPayloadSize = optional.Some(_g.PayloadSize())
	}
	var _dPayloadSize uint8
	if _mPayload.IsPresent() {
		_dPayloadSize = uint8(_mPayload.Proven().Len())
	} else if _s, ok := _mPayloadSize.Get(); ok {
		_dPayloadSize = _s
	}
	_cFlags := src.Flags()
	_cFlags &^= 0x1
	if _mPayload.IsPresent() {
		_cFlags |= 0x1
	}
	if err := w.WriteBytes([]byte{byte(_cFlags)}); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{byte(_dPayloadSize)}); err != nil {
		return err
	}
	if _mPayload.IsPresent() {
		if err := w.WriteBytes(_mPayload.Proven().Bytes()); err != nil {
			return err
		}
	}
	return nil
}

// DecodeCodecPresentIfLengthRef decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecPresentIfLengthRef(cursor *codec.SceCursor) (*CodecPresentIfLengthRef, error) {
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
	var Flags uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Flags = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var PayloadSize uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		PayloadSize = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var Payload optional.Of[[]byte]
	if (Flags & 0x01) != 0 {
		_len := PayloadSize
		if _len > 32 {
			return nil, codec.ErrLengthOutOfRange
		}
		_n := int(_len)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Payload = optional.Some(append([]byte(nil), raw...))
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	_bPayload, err := codec.MakeOptionalBytes[CodecPresentIfLengthRefPayloadBound](Payload)
	if err != nil {
		return nil, err
	}
	var _gFlagsHasPayload CodecPresentIfLengthRefFlagsHasPayload
	if ((Flags >> 0) & 1) != 0 {
		_gFlagsHasPayload = MakeCodecPresentIfLengthRefFlagsHasPayloadHasPayload(_bPayload.Proven())
	} else {
		_gFlagsHasPayload = MakeCodecPresentIfLengthRefFlagsHasPayloadNotHasPayload(PayloadSize)
	}
	return &CodecPresentIfLengthRef{
		flags: Flags,
		flagsHasPayload: _gFlagsHasPayload,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecPresentIfLengthRef) HasPayload() bool {
	return (s.flags & 0x01) != 0
}

func (s *CodecPresentIfLengthRef) SetHasPayload(v bool) {
	if v {
		s.flags |= 0x01
	} else {
		s.flags &^= 0x01
	}
}

// Encode writes the CodecPresentIfLengthRef into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecPresentIfLengthRef) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecPresentIfLengthRef) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 34)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
