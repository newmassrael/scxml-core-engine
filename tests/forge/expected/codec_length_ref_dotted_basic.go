// SCE-MAP: codec_length_ref_dotted_basic:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_length_ref_dotted_basic

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecLengthRefDottedBasic represents the codec frame layout.
type CodecLengthRefDottedBasic struct {
	Carrier uint8
	Payload []byte
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
	Payload := raw[1:1+int((Carrier >> 4) & 0xF)]
	value := &CodecLengthRefDottedBasic{
		Carrier: Carrier,
		Payload: Payload,
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
	return uint8((s.Carrier >> 0) & 0x0F)
}

func (s *CodecLengthRefDottedBasic) SetHdr(v uint8) {
	const _shiftedMask uint8 = 0x0F << 0
	_val := (uint8(v) & 0x0F) << 0
	s.Carrier = (s.Carrier &^ _shiftedMask) | _val
}

func (s *CodecLengthRefDottedBasic) PayloadLen() uint8 {
	return uint8((s.Carrier >> 4) & 0x0F)
}

func (s *CodecLengthRefDottedBasic) SetPayloadLen(v uint8) {
	const _shiftedMask uint8 = 0x0F << 4
	_val := (uint8(v) & 0x0F) << 4
	s.Carrier = (s.Carrier &^ _shiftedMask) | _val
}

// Encode writes the CodecLengthRefDottedBasic into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecLengthRefDottedBasic) Encode(w codec.SceSink) error {
	if err := w.WriteBytes([]byte{ byte(s.Carrier) }); err != nil {
		return err
	}
	if err := w.WriteBytes(s.Payload); err != nil {
		return err
	}
	return nil
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
