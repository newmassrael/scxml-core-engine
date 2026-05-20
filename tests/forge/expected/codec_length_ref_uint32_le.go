// SCE-MAP: codec_length_ref_uint32_le:13

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_length_ref_uint32_le

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecLengthRefUint32Le represents the codec frame layout.
type CodecLengthRefUint32Le struct {
	PayloadLen uint32
	Payload []byte
}

// DecodeCodecLengthRefUint32Le decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecLengthRefUint32Le(cursor *codec.SceCursor) (*CodecLengthRefUint32Le, error) {
	frameLen := cursor.Remaining()
	if frameLen < 4 {
		return nil, codec.ErrNeedMoreBytes
	}
	raw, err := cursor.PeekSlice(frameLen)
	if err != nil {
		return nil, err
	}
	PayloadLen := uint32(raw[0]) | uint32(raw[1])<<8 | uint32(raw[2])<<16 | uint32(raw[3])<<24
	Payload := raw[4:4+int(PayloadLen)]
	value := &CodecLengthRefUint32Le{
		PayloadLen: PayloadLen,
		Payload: Payload,
	}
	if err := cursor.Advance(frameLen); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode serializes the CodecLengthRefUint32Le into raw bytes.
func (s *CodecLengthRefUint32Le) Encode() []byte {
	r := make([]byte, 0, 1028)
	r = append(r, byte(s.PayloadLen & 0xFF))
	r = append(r, byte(s.PayloadLen >> 8 & 0xFF))
	r = append(r, byte(s.PayloadLen >> 16 & 0xFF))
	r = append(r, byte(s.PayloadLen >> 24 & 0xFF))
	r = append(r, s.Payload...)
	return r
}
