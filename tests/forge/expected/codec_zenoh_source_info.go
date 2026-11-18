// SCE-MAP: codec_zenoh_source_info:57 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_source_info

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohSourceInfoZidBound is the bound of the zid element of CodecZenohSourceInfo: 1 to 16 bytes
// (docs/adr/0077 §3). A Bytes of it is made only within it.
type CodecZenohSourceInfoZidBound struct{}

// Least is the fewest bytes the element holds.
func (CodecZenohSourceInfoZidBound) Least() int { return 1 }

// Most is the most bytes the element holds.
func (CodecZenohSourceInfoZidBound) Most() int { return 16 }

// CodecZenohSourceInfoSource is what the encode of CodecZenohSourceInfo reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohSourceInfo is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohSourceInfoSource interface {
	Header() uint8
	Zid() codec.Bytes[CodecZenohSourceInfoZidBound]
	Eid() uint32
	Sn() uint32
}

// CodecZenohSourceInfo represents the codec frame layout.
type CodecZenohSourceInfo struct {
	header uint8
	zid codec.Bytes[CodecZenohSourceInfoZidBound]
	eid uint32
	sn uint32
}

// MakeCodecZenohSourceInfo returns the CodecZenohSourceInfo that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohSourceInfo, or as its zero value, and is read through its methods.
func MakeCodecZenohSourceInfo(header uint8, zid codec.Bytes[CodecZenohSourceInfoZidBound], eid uint32, sn uint32) CodecZenohSourceInfo {
	return CodecZenohSourceInfo{header: header, zid: zid, eid: eid, sn: sn}
}

// Header returns the element the value presents as Header.
func (s *CodecZenohSourceInfo) Header() uint8 {
	return s.header
}

// Zid returns the element the value presents as Zid.
func (s *CodecZenohSourceInfo) Zid() codec.Bytes[CodecZenohSourceInfoZidBound] {
	return s.zid
}

// Eid returns the element the value presents as Eid.
func (s *CodecZenohSourceInfo) Eid() uint32 {
	return s.eid
}

// Sn returns the element the value presents as Sn.
func (s *CodecZenohSourceInfo) Sn() uint32 {
	return s.sn
}

// EncodeSource writes any source of CodecZenohSourceInfo into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohSourceInfoSource, w codec.SceSink) error {
	_cHeader := src.Header()
	_cHeader = (_cHeader &^ 0xF0) | (uint8(src.Zid().Len() - 1) << 4)
	if err := w.WriteBytes([]byte{byte(_cHeader)}); err != nil {
		return err
	}
	if err := w.WriteBytes(src.Zid().Bytes()); err != nil {
		return err
	}
	if err := codec.WriteVLEU32(w, uint32(src.Eid())); err != nil {
		return err
	}
	if err := codec.WriteVLEU32(w, uint32(src.Sn())); err != nil {
		return err
	}
	return nil
}

// DecodeCodecZenohSourceInfo decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohSourceInfo(cursor *codec.SceCursor) (*CodecZenohSourceInfo, error) {
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
		_len := int((Header >> 4) & 0xF)
		_n := _len + 1
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
	_bZid, err := codec.MakeBytes[CodecZenohSourceInfoZidBound](Zid)
	if err != nil {
		return nil, err
	}
	return &CodecZenohSourceInfo{
		header: Header,
		zid: _bZid,
		eid: Eid,
		sn: Sn,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohSourceInfo) ZidlenM1() uint8 {
	return uint8((s.header >> 4) & 0x0F)
}

func (s *CodecZenohSourceInfo) SetZidlenM1(v uint8) {
	const _shiftedMask uint8 = 0x0F << 4
	_val := (uint8(v) & 0x0F) << 4
	s.header = (s.header &^ _shiftedMask) | _val
}

// Encode writes the CodecZenohSourceInfo into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohSourceInfo) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohSourceInfo) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 27)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
