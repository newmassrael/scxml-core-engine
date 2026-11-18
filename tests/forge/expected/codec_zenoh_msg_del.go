// SCE-MAP: codec_zenoh_msg_del:53 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_msg_del

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
	"example.com/sce-forge/codec_zenoh_timestamp"
	"example.com/sce-forge/codec_zenoh_ext_entry"
)

// CodecZenohMsgDelExtensionsBound is the bound of the extensions element of CodecZenohMsgDel: 1 or more entries, the protocol stating no most
// (docs/adr/0065 §6 R2). A List of it is made only within it.
type CodecZenohMsgDelExtensionsBound struct{}

// Least is the fewest entries the element holds.
func (CodecZenohMsgDelExtensionsBound) Least() int { return 1 }

// Most is the most entries the element holds.
func (CodecZenohMsgDelExtensionsBound) Most() int { return codec.Unbounded }

// CodecZenohMsgDelSource is what the encode of CodecZenohMsgDel reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohMsgDel is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohMsgDelSource interface {
	Header() uint8
	Timestamp() optional.Of[codec_zenoh_timestamp.CodecZenohTimestampSource]
	Extensions() optional.Of[codec.List[CodecZenohMsgDelExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]]
}

// CodecZenohMsgDel represents the codec frame layout.
type CodecZenohMsgDel struct {
	header uint8
	timestamp optional.Of[codec_zenoh_timestamp.CodecZenohTimestampSource]
	extensions optional.Of[codec.List[CodecZenohMsgDelExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]]
}

// MakeCodecZenohMsgDel returns the CodecZenohMsgDel that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohMsgDel, or as its zero value, and is read through its methods.
func MakeCodecZenohMsgDel(header uint8, timestamp optional.Of[codec_zenoh_timestamp.CodecZenohTimestampSource], extensions optional.Of[codec.List[CodecZenohMsgDelExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]]) CodecZenohMsgDel {
	return CodecZenohMsgDel{header: header, timestamp: timestamp, extensions: extensions}
}

// Header returns the element the value presents as Header.
func (s *CodecZenohMsgDel) Header() uint8 {
	return s.header
}

// Timestamp returns the element the value presents as Timestamp.
func (s *CodecZenohMsgDel) Timestamp() optional.Of[codec_zenoh_timestamp.CodecZenohTimestampSource] {
	return s.timestamp
}

// Extensions returns the element the value presents as Extensions.
func (s *CodecZenohMsgDel) Extensions() optional.Of[codec.List[CodecZenohMsgDelExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]] {
	return s.extensions
}

// CodecZenohMsgDelOverrides holds the one-bit flags of CodecZenohMsgDel a parent may
// set in place of the ones a value presents, such as the flag a chain whose
// entries are CodecZenohMsgDel ends them on, which the chain derives from where
// each entry stands. A flag it leaves absent is the one the value presents.
type CodecZenohMsgDelOverrides struct {
	// HeaderX is the `X` flag of `header`, where a parent sets it.
	HeaderX optional.Of[bool]
}

// EncodeSource writes any source of CodecZenohMsgDel into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohMsgDelSource, w codec.SceSink) error {
	return EncodeSourceOverriding(src, w, CodecZenohMsgDelOverrides{})
}

// EncodeSourceOverriding is EncodeSource, each flag overrides sets written in
// place of the one src presents.
func EncodeSourceOverriding(src CodecZenohMsgDelSource, w codec.SceSink, overrides CodecZenohMsgDelOverrides) error {
	_cHeader := src.Header()
	_cHeader &^= 0x20
	if src.Timestamp().IsPresent() {
		_cHeader |= 0x20
	}
	_cHeader &^= 0x80
	if src.Extensions().IsPresent() {
		_cHeader |= 0x80
	}
	if _o, ok := overrides.HeaderX.Get(); ok {
		_cHeader &^= 0x40
		if _o {
			_cHeader |= 0x40
		}
	}
	if err := w.WriteBytes([]byte{byte(_cHeader)}); err != nil {
		return err
	}
	if src.Timestamp().IsPresent() {
		if err := codec_zenoh_timestamp.EncodeSource(src.Timestamp().Proven(), w); err != nil {
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
	return nil
}

// NewCodecZenohMsgDel returns a CodecZenohMsgDel initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecZenohMsgDel().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecZenohMsgDel{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecZenohMsgDel() *CodecZenohMsgDel {
	return &CodecZenohMsgDel{
		header: uint8(0x02),
	}
}

// DecodeCodecZenohMsgDel decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohMsgDel(cursor *codec.SceCursor) (*CodecZenohMsgDel, error) {
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
	var Timestamp optional.Of[codec_zenoh_timestamp.CodecZenohTimestamp]
	if (Header & 0x20) != 0 {
		_emb, err := codec_zenoh_timestamp.DecodeCodecZenohTimestamp(cursor)
		if err != nil {
			return nil, err
		}
		Timestamp = optional.Some(*_emb)
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
	_lExtensions, err := codec.MakeOptionalList[CodecZenohMsgDelExtensionsBound](codec.OptionalSourcesOf(Extensions, func(e *codec_zenoh_ext_entry.CodecZenohExtEntry) codec_zenoh_ext_entry.CodecZenohExtEntrySource { return e }))
	if err != nil {
		return nil, err
	}
	return &CodecZenohMsgDel{
		header: Header,
		timestamp: optional.Convert(Timestamp, func(b codec_zenoh_timestamp.CodecZenohTimestamp) codec_zenoh_timestamp.CodecZenohTimestampSource { return &b }),
		extensions: _lExtensions,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohMsgDel) Mid() uint8 {
	return uint8((s.header >> 0) & 0x1F)
}

func (s *CodecZenohMsgDel) SetMid(v uint8) {
	const _shiftedMask uint8 = 0x1F << 0
	_val := (uint8(v) & 0x1F) << 0
	s.header = (s.header &^ _shiftedMask) | _val
}

func (s *CodecZenohMsgDel) T() bool {
	return (s.header & 0x20) != 0
}

func (s *CodecZenohMsgDel) SetT(v bool) {
	if v {
		s.header |= 0x20
	} else {
		s.header &^= 0x20
	}
}

func (s *CodecZenohMsgDel) X() bool {
	return (s.header & 0x40) != 0
}

func (s *CodecZenohMsgDel) SetX(v bool) {
	if v {
		s.header |= 0x40
	} else {
		s.header &^= 0x40
	}
}

func (s *CodecZenohMsgDel) Z() bool {
	return (s.header & 0x80) != 0
}

func (s *CodecZenohMsgDel) SetZ(v bool) {
	if v {
		s.header |= 0x80
	} else {
		s.header &^= 0x80
	}
}

// Encode writes the CodecZenohMsgDel into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohMsgDel) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohMsgDel) EncodeToBytes() []byte {
	_dst := make([]byte, 0)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
