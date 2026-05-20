// SCE-MAP: codec_length_ref_uint16_be:12

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_length_ref_uint16_be

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecLengthRefUint16Be represents the codec frame layout.
type CodecLengthRefUint16Be struct {
	PayloadLen uint16
	Payload []byte
}

// DecodeCodecLengthRefUint16Be decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecLengthRefUint16Be(cursor *codec.SceCursor) (*CodecLengthRefUint16Be, error) {
	frameLen := cursor.Remaining()
	if frameLen < 2 {
		return nil, codec.ErrNeedMoreBytes
	}
	raw, err := cursor.PeekSlice(frameLen)
	if err != nil {
		return nil, err
	}
	PayloadLen := uint16(raw[0])<<8 | uint16(raw[1])
	Payload := raw[2:2+int(PayloadLen)]
	value := &CodecLengthRefUint16Be{
		PayloadLen: PayloadLen,
		Payload: Payload,
	}
	if err := cursor.Advance(frameLen); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode serializes the CodecLengthRefUint16Be into raw bytes.
func (s *CodecLengthRefUint16Be) Encode() []byte {
	r := make([]byte, 0, 1026)
	r = append(r, byte(s.PayloadLen >> 8 & 0xFF))
	r = append(r, byte(s.PayloadLen & 0xFF))
	r = append(r, s.Payload...)
	return r
}
