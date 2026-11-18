// SCE-MAP: codec_zenoh_err:57 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_err

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
	"example.com/sce-forge/codec_zenoh_encoding"
	"example.com/sce-forge/codec_zenoh_ext_entry"
)

// CodecZenohErrExtensionsBound is the bound of the extensions element of CodecZenohErr: 1 or more entries, the protocol stating no most
// (docs/adr/0065 §6 R2). A List of it is made only within it.
type CodecZenohErrExtensionsBound struct{}

// Least is the fewest entries the element holds.
func (CodecZenohErrExtensionsBound) Least() int { return 1 }

// Most is the most entries the element holds.
func (CodecZenohErrExtensionsBound) Most() int { return codec.Unbounded }

// CodecZenohErrPayloadBound is the bound of the payload element of CodecZenohErr: 0 to 4294967295 bytes
// (docs/adr/0077 §3). A Bytes of it is made only within it.
type CodecZenohErrPayloadBound struct{}

// Least is the fewest bytes the element holds.
func (CodecZenohErrPayloadBound) Least() int { return 0 }

// Most is the most bytes the element holds.
func (CodecZenohErrPayloadBound) Most() int { return 4294967295 }

// CodecZenohErrSource is what the encode of CodecZenohErr reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohErr is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohErrSource interface {
	Header() uint8
	Encoding() optional.Of[codec_zenoh_encoding.CodecZenohEncodingSource]
	Extensions() optional.Of[codec.List[CodecZenohErrExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]]
	Payload() codec.Bytes[CodecZenohErrPayloadBound]
}

// CodecZenohErr represents the codec frame layout.
type CodecZenohErr struct {
	header uint8
	encoding optional.Of[codec_zenoh_encoding.CodecZenohEncodingSource]
	extensions optional.Of[codec.List[CodecZenohErrExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]]
	payload codec.Bytes[CodecZenohErrPayloadBound]
}

// MakeCodecZenohErr returns the CodecZenohErr that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohErr, or as its zero value, and is read through its methods.
func MakeCodecZenohErr(header uint8, encoding optional.Of[codec_zenoh_encoding.CodecZenohEncodingSource], extensions optional.Of[codec.List[CodecZenohErrExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]], payload codec.Bytes[CodecZenohErrPayloadBound]) CodecZenohErr {
	return CodecZenohErr{header: header, encoding: encoding, extensions: extensions, payload: payload}
}

// Header returns the element the value presents as Header.
func (s *CodecZenohErr) Header() uint8 {
	return s.header
}

// Encoding returns the element the value presents as Encoding.
func (s *CodecZenohErr) Encoding() optional.Of[codec_zenoh_encoding.CodecZenohEncodingSource] {
	return s.encoding
}

// Extensions returns the element the value presents as Extensions.
func (s *CodecZenohErr) Extensions() optional.Of[codec.List[CodecZenohErrExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]] {
	return s.extensions
}

// Payload returns the element the value presents as Payload.
func (s *CodecZenohErr) Payload() codec.Bytes[CodecZenohErrPayloadBound] {
	return s.payload
}

// WirePayloadLen returns the payload_len the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecZenohErr) WirePayloadLen() uint64 {
	return uint64(s.Payload().Len())
}

// CodecZenohErrOverrides holds the one-bit flags of CodecZenohErr a parent may
// set in place of the ones a value presents, such as the flag a chain whose
// entries are CodecZenohErr ends them on, which the chain derives from where
// each entry stands. A flag it leaves absent is the one the value presents.
type CodecZenohErrOverrides struct {
	// HeaderX is the `X` flag of `header`, where a parent sets it.
	HeaderX optional.Of[bool]
}

// EncodeSource writes any source of CodecZenohErr into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives payload_len from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohErrSource, w codec.SceSink) error {
	return EncodeSourceOverriding(src, w, CodecZenohErrOverrides{})
}

// EncodeSourceOverriding is EncodeSource, each flag overrides sets written in
// place of the one src presents.
func EncodeSourceOverriding(src CodecZenohErrSource, w codec.SceSink, overrides CodecZenohErrOverrides) error {
	_dPayloadLen := uint64(src.Payload().Len())
	_cHeader := src.Header()
	_cHeader &^= 0x40
	if src.Encoding().IsPresent() {
		_cHeader |= 0x40
	}
	_cHeader &^= 0x80
	if src.Extensions().IsPresent() {
		_cHeader |= 0x80
	}
	if _o, ok := overrides.HeaderX.Get(); ok {
		_cHeader &^= 0x20
		if _o {
			_cHeader |= 0x20
		}
	}
	if err := w.WriteBytes([]byte{byte(_cHeader)}); err != nil {
		return err
	}
	if src.Encoding().IsPresent() {
		if err := codec_zenoh_encoding.EncodeSource(src.Encoding().Proven(), w); err != nil {
			return err
		}
	}
	if src.Extensions().IsPresent() {
		{
			_leftExtensions := src.Extensions().Proven().Len()
			for _e := range src.Extensions().Proven().All() {
				_leftExtensions--
				if err := codec_zenoh_ext_entry.EncodeSourceOverriding(_e, w, codec_zenoh_ext_entry.CodecZenohExtEntryOverrides{HeaderZ: optional.Some(_leftExtensions != 0)}); err != nil {
					return err
				}
			}
		}
	}
	if err := codec.WriteVLEU64(w, uint64(_dPayloadLen)); err != nil {
		return err
	}
	if err := w.WriteBytes(src.Payload().Bytes()); err != nil {
		return err
	}
	return nil
}

// NewCodecZenohErr returns a CodecZenohErr initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecZenohErr().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecZenohErr{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecZenohErr() *CodecZenohErr {
	return &CodecZenohErr{
		header: uint8(0x05),
	}
}

// DecodeCodecZenohErr decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohErr(cursor *codec.SceCursor) (*CodecZenohErr, error) {
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
	var Encoding optional.Of[codec_zenoh_encoding.CodecZenohEncoding]
	if (Header & 0x40) != 0 {
		_emb, err := codec_zenoh_encoding.DecodeCodecZenohEncoding(cursor)
		if err != nil {
			return nil, err
		}
		Encoding = optional.Some(*_emb)
	}
	var Extensions optional.Of[[]codec_zenoh_ext_entry.CodecZenohExtEntry]
	if (Header & 0x80) != 0 {
		_list := make([]codec_zenoh_ext_entry.CodecZenohExtEntry, 0)
		_more := false
		for {
			_elem, err := codec_zenoh_ext_entry.DecodeCodecZenohExtEntry(cursor)
			if err != nil {
				return nil, err
			}
			_more = _elem.Z()
			_list = append(_list, *_elem)
			if !_more {
				break
			}
		}
		if _more && cursor.Remaining() == 0 {
			return nil, codec.ErrNeedMoreBytes
		}
		Extensions = optional.Some(_list)
	}
	PayloadLen, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
	var Payload []byte
	{
		_len := PayloadLen
		if _len > 4294967295 {
			return nil, codec.ErrLengthOutOfRange
		}
		if uint64(_len) > uint64(^uint(0)>>1)-0 {
			return nil, codec.ErrNeedMoreBytes
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
	_lExtensions, err := codec.MakeOptionalList[CodecZenohErrExtensionsBound](codec.OptionalSourcesOf(Extensions, func(e *codec_zenoh_ext_entry.CodecZenohExtEntry) codec_zenoh_ext_entry.CodecZenohExtEntrySource { return e }))
	if err != nil {
		return nil, err
	}
	_bPayload, err := codec.MakeBytes[CodecZenohErrPayloadBound](Payload)
	if err != nil {
		return nil, err
	}
	return &CodecZenohErr{
		header: Header,
		encoding: optional.Convert(Encoding, func(b codec_zenoh_encoding.CodecZenohEncoding) codec_zenoh_encoding.CodecZenohEncodingSource { return &b }),
		extensions: _lExtensions,
		payload: _bPayload,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohErr) Mid() uint8 {
	return uint8((s.header >> 0) & 0x1F)
}

func (s *CodecZenohErr) SetMid(v uint8) {
	const _shiftedMask uint8 = 0x1F << 0
	_val := (uint8(v) & 0x1F) << 0
	s.header = (s.header &^ _shiftedMask) | _val
}

func (s *CodecZenohErr) X() bool {
	return (s.header & 0x20) != 0
}

func (s *CodecZenohErr) SetX(v bool) {
	if v {
		s.header |= 0x20
	} else {
		s.header &^= 0x20
	}
}

func (s *CodecZenohErr) E() bool {
	return (s.header & 0x40) != 0
}

func (s *CodecZenohErr) SetE(v bool) {
	if v {
		s.header |= 0x40
	} else {
		s.header &^= 0x40
	}
}

func (s *CodecZenohErr) Z() bool {
	return (s.header & 0x80) != 0
}

func (s *CodecZenohErr) SetZ(v bool) {
	if v {
		s.header |= 0x80
	} else {
		s.header &^= 0x80
	}
}

// Encode writes the CodecZenohErr into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohErr) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohErr) EncodeToBytes() []byte {
	_dst := make([]byte, 0)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
