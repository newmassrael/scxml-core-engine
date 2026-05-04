// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_source_info

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohSourceInfo represents the codec frame layout.
type CodecZenohSourceInfo struct {
	Header uint8
	Zid []byte
	Eid uint32
	Sn uint32
}

// DecodeCodecZenohSourceInfo decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohSourceInfo(cursor *codec.SceCursor) (*CodecZenohSourceInfo, error) {
	// Streaming codec: each field reads from cursor directly
	// (VLE base-128 chain). Local var name reuses the Go-PascalCase
	// `field.id` — the struct literal's `Foo: Foo` is unambiguous
	// because the package owns both names. RFC §5.B B4: per-field
	// bit-size dispatch routes Fixed / LengthRef siblings of VLE
	// fields through `present_if_decode_stmt` (predicate=None arms).
	// Pure-VLE codecs stay byte-stable.
	var Header uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Header = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var Zid []byte
	{
		_n := (int((Header >> 4) & 0xF) + 1)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Zid = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	Eid, err := cursor.ReadVLEU32()
	if err != nil { return nil, err }
	Sn, err := cursor.ReadVLEU32()
	if err != nil { return nil, err }
	return &CodecZenohSourceInfo{
		Header: Header,
		Zid: Zid,
		Eid: Eid,
		Sn: Sn,
	}, nil
}

// RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohSourceInfo) ZidlenM1() uint8 {
	return uint8((s.Header >> 4) & 0x0F)
}

func (s *CodecZenohSourceInfo) SetZidlenM1(v uint8) {
	const _shiftedMask uint8 = 0x0F << 4
	_val := (uint8(v) & 0x0F) << 4
	s.Header = (s.Header &^ _shiftedMask) | _val
}

// Encode serializes the CodecZenohSourceInfo into raw bytes.
func (s *CodecZenohSourceInfo) Encode() []byte {
	// RFC §5.B B4: per-field bit-size dispatch routes Fixed /
	// LengthRef siblings of VLE fields through
	// `present_if_encode_block` (predicate=None arms). Pure-VLE
	// codecs stay byte-stable.
	r := make([]byte, 0, 27)
	r = append(r, s.Header)
	r = append(r, s.Zid...)
	{
		_w := uint64(s.Eid)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	{
		_w := uint64(s.Sn)
		for _w >= 0x80 {
			r = append(r, byte(_w&0x7F)|0x80)
			_w >>= 7
		}
		r = append(r, byte(_w))
	}
	return r
}
