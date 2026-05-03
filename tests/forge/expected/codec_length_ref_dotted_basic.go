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
// is shorter than the declared minimum frame (RFC §5.B L494-519).
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
	value := &CodecLengthRefDottedBasic{
		Carrier: raw[0],
		Payload: raw[1:1+int((raw[0] >> 4) & 0xF)],
	}
	if err := cursor.Advance(frameLen); err != nil {
		return nil, err
	}
	return value, nil
}

// RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
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

// Encode serializes the CodecLengthRefDottedBasic into raw bytes.
func (s *CodecLengthRefDottedBasic) Encode() []byte {
	r := make([]byte, 0, 16)
	r = append(r, byte(s.Carrier))
	r = append(r, s.Payload...)
	return r
}
