// SCE-MAP: codec_tlv_entry:10

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_tlv_entry

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecTlvEntry represents the codec frame layout.
type CodecTlvEntry struct {
	EntryType uint8
	EntryLen uint8
	EntryBody []byte
}

// DecodeCodecTlvEntry decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecTlvEntry(cursor *codec.SceCursor) (*CodecTlvEntry, error) {
	frameLen := cursor.Remaining()
	if frameLen < 2 {
		return nil, codec.ErrNeedMoreBytes
	}
	raw, err := cursor.PeekSlice(frameLen)
	if err != nil {
		return nil, err
	}
	value := &CodecTlvEntry{
		EntryType: raw[0],
		EntryLen: raw[1],
		EntryBody: raw[2:2+int(raw[1])],
	}
	if err := cursor.Advance(frameLen); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode serializes the CodecTlvEntry into raw bytes.
func (s *CodecTlvEntry) Encode() []byte {
	r := make([]byte, 0, 34)
	r = append(r, byte(s.EntryType))
	r = append(r, byte(s.EntryLen))
	r = append(r, s.EntryBody...)
	return r
}
