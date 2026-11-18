// SCE-MAP: codec_length_ref_dotted_basic:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_length_ref_dotted_basic

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecLengthRefDottedBasicPayloadBound is the bound of the payload element of CodecLengthRefDottedBasic: 0 to 15 bytes
// (docs/adr/0077 §3). A Bytes of it is made only within it.
type CodecLengthRefDottedBasicPayloadBound struct{}

// Least is the fewest bytes the element holds.
func (CodecLengthRefDottedBasicPayloadBound) Least() int { return 0 }

// Most is the most bytes the element holds.
func (CodecLengthRefDottedBasicPayloadBound) Most() int { return 15 }

// CodecLengthRefDottedBasicSource is what the encode of CodecLengthRefDottedBasic reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecLengthRefDottedBasic is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecLengthRefDottedBasicSource interface {
	Carrier() uint8
	Payload() codec.Bytes[CodecLengthRefDottedBasicPayloadBound]
}

// CodecLengthRefDottedBasic represents the codec frame layout.
type CodecLengthRefDottedBasic struct {
	carrier uint8
	payload codec.Bytes[CodecLengthRefDottedBasicPayloadBound]
}

// MakeCodecLengthRefDottedBasic returns the CodecLengthRefDottedBasic that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecLengthRefDottedBasic, or as its zero value, and is read through its methods.
func MakeCodecLengthRefDottedBasic(carrier uint8, payload codec.Bytes[CodecLengthRefDottedBasicPayloadBound]) CodecLengthRefDottedBasic {
	return CodecLengthRefDottedBasic{carrier: carrier, payload: payload}
}

// Carrier returns the element the value presents as Carrier.
func (s *CodecLengthRefDottedBasic) Carrier() uint8 {
	return s.carrier
}

// Payload returns the element the value presents as Payload.
func (s *CodecLengthRefDottedBasic) Payload() codec.Bytes[CodecLengthRefDottedBasicPayloadBound] {
	return s.payload
}

// EncodeSource writes any source of CodecLengthRefDottedBasic into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecLengthRefDottedBasicSource, w codec.SceSink) error {
	_cCarrier := src.Carrier()
	_cCarrier = (_cCarrier &^ 0xF0) | (uint8(src.Payload().Len()) << 4)
	if err := w.WriteBytes([]byte{byte(_cCarrier)}); err != nil {
		return err
	}
	if err := w.WriteBytes(src.Payload().Bytes()); err != nil {
		return err
	}
	return nil
}

// DecodeCodecLengthRefDottedBasic decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecLengthRefDottedBasic(cursor *codec.SceCursor) (*CodecLengthRefDottedBasic, error) {
	frameLen := cursor.Remaining()
	if frameLen < 1 {
		return nil, codec.ErrNeedMoreBytes
	}
	raw, err := cursor.PeekSlice(frameLen)
	if err != nil {
		return nil, err
	}
	Carrier := raw[0]
	if frameLen < 1+int((Carrier >> 4) & 0xF) {
		return nil, codec.ErrNeedMoreBytes
	}
	Payload := raw[1:1+int((Carrier >> 4) & 0xF)]
	_bPayload, err := codec.MakeBytes[CodecLengthRefDottedBasicPayloadBound](Payload)
	if err != nil {
		return nil, err
	}
	value := &CodecLengthRefDottedBasic{
		carrier: Carrier,
		payload: _bPayload,
	}
	if err := cursor.Advance(frameLen); err != nil {
		return nil, err
	}
	return value, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecLengthRefDottedBasic) Hdr() uint8 {
	return uint8((s.carrier >> 0) & 0x0F)
}

func (s *CodecLengthRefDottedBasic) SetHdr(v uint8) {
	const _shiftedMask uint8 = 0x0F << 0
	_val := (uint8(v) & 0x0F) << 0
	s.carrier = (s.carrier &^ _shiftedMask) | _val
}

func (s *CodecLengthRefDottedBasic) PayloadLen() uint8 {
	return uint8((s.carrier >> 4) & 0x0F)
}

func (s *CodecLengthRefDottedBasic) SetPayloadLen(v uint8) {
	const _shiftedMask uint8 = 0x0F << 4
	_val := (uint8(v) & 0x0F) << 4
	s.carrier = (s.carrier &^ _shiftedMask) | _val
}

// Encode writes the CodecLengthRefDottedBasic into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecLengthRefDottedBasic) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecLengthRefDottedBasic) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 16)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
