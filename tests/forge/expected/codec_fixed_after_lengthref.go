// SCE-MAP: codec_fixed_after_lengthref:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_fixed_after_lengthref

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecFixedAfterLengthrefPayloadBound is the bound of the payload element of CodecFixedAfterLengthref: 0 to 1500 bytes
// (docs/adr/0077 §3). A Bytes of it is made only within it.
type CodecFixedAfterLengthrefPayloadBound struct{}

// Least is the fewest bytes the element holds.
func (CodecFixedAfterLengthrefPayloadBound) Least() int { return 0 }

// Most is the most bytes the element holds.
func (CodecFixedAfterLengthrefPayloadBound) Most() int { return 1500 }

// CodecFixedAfterLengthrefSource is what the encode of CodecFixedAfterLengthref reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecFixedAfterLengthref is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecFixedAfterLengthrefSource interface {
	Header() uint8
	Payload() codec.Bytes[CodecFixedAfterLengthrefPayloadBound]
	Crc32() uint32
}

// CodecFixedAfterLengthref represents the codec frame layout.
type CodecFixedAfterLengthref struct {
	header uint8
	payload codec.Bytes[CodecFixedAfterLengthrefPayloadBound]
	crc32 uint32
}

// MakeCodecFixedAfterLengthref returns the CodecFixedAfterLengthref that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecFixedAfterLengthref, or as its zero value, and is read through its methods.
func MakeCodecFixedAfterLengthref(header uint8, payload codec.Bytes[CodecFixedAfterLengthrefPayloadBound], crc32 uint32) CodecFixedAfterLengthref {
	return CodecFixedAfterLengthref{header: header, payload: payload, crc32: crc32}
}

// Header returns the element the value presents as Header.
func (s *CodecFixedAfterLengthref) Header() uint8 {
	return s.header
}

// Payload returns the element the value presents as Payload.
func (s *CodecFixedAfterLengthref) Payload() codec.Bytes[CodecFixedAfterLengthrefPayloadBound] {
	return s.payload
}

// Crc32 returns the element the value presents as Crc32.
func (s *CodecFixedAfterLengthref) Crc32() uint32 {
	return s.crc32
}

// WirePayloadLen returns the payload_len the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecFixedAfterLengthref) WirePayloadLen() uint16 {
	return uint16(s.Payload().Len())
}

// EncodeSource writes any source of CodecFixedAfterLengthref into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives payload_len from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecFixedAfterLengthrefSource, w codec.SceSink) error {
	_dPayloadLen := uint16(src.Payload().Len())
	if err := w.WriteBytes([]byte{byte(src.Header())}); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{byte(_dPayloadLen & 0xFF), byte(_dPayloadLen >> 8 & 0xFF)}); err != nil {
		return err
	}
	if err := w.WriteBytes(src.Payload().Bytes()); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{byte(src.Crc32() & 0xFF), byte(src.Crc32() >> 8 & 0xFF), byte(src.Crc32() >> 16 & 0xFF), byte(src.Crc32() >> 24 & 0xFF)}); err != nil {
		return err
	}
	return nil
}

// DecodeCodecFixedAfterLengthref decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecFixedAfterLengthref(cursor *codec.SceCursor) (*CodecFixedAfterLengthref, error) {
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
	var PayloadLen uint16
	{
		raw, err := cursor.PeekSlice(2)
		if err != nil {
			return nil, err
		}
		PayloadLen = uint16(raw[0]) | uint16(raw[1])<<8
		if err := cursor.Advance(2); err != nil {
			return nil, err
		}
	}
	var Payload []byte
	{
		_len := PayloadLen
		if _len > 1500 {
			return nil, codec.ErrLengthOutOfRange
		}
		_n := int(_len)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Payload = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	var Crc32 uint32
	{
		raw, err := cursor.PeekSlice(4)
		if err != nil {
			return nil, err
		}
		Crc32 = uint32(raw[0]) | uint32(raw[1])<<8 | uint32(raw[2])<<16 | uint32(raw[3])<<24
		if err := cursor.Advance(4); err != nil {
			return nil, err
		}
	}
	_bPayload, err := codec.MakeBytes[CodecFixedAfterLengthrefPayloadBound](Payload)
	if err != nil {
		return nil, err
	}
	return &CodecFixedAfterLengthref{
		header: Header,
		payload: _bPayload,
		crc32: Crc32,
	}, nil
}

// Encode writes the CodecFixedAfterLengthref into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecFixedAfterLengthref) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecFixedAfterLengthref) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 1508)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
