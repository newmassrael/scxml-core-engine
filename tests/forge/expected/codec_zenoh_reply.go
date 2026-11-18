// SCE-MAP: codec_zenoh_reply:54 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_reply

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
	"example.com/sce-forge/codec_zenoh_ext_entry"
	"example.com/sce-forge/codec_zenoh_push_body"
)

// CodecZenohReplyExtensionsBound is the bound of the extensions element of CodecZenohReply: 1 or more entries, the protocol stating no most
// (docs/adr/0065 §6 R2). A List of it is made only within it.
type CodecZenohReplyExtensionsBound struct{}

// Least is the fewest entries the element holds.
func (CodecZenohReplyExtensionsBound) Least() int { return 1 }

// Most is the most entries the element holds.
func (CodecZenohReplyExtensionsBound) Most() int { return codec.Unbounded }

// CodecZenohReplySource is what the encode of CodecZenohReply reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohReply is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohReplySource interface {
	Header() uint8
	Consolidation() optional.Of[uint8]
	Extensions() optional.Of[codec.List[CodecZenohReplyExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]]
	Body() codec_zenoh_push_body.CodecZenohPushBodySource
}

// CodecZenohReply represents the codec frame layout.
type CodecZenohReply struct {
	header uint8
	consolidation optional.Of[uint8]
	extensions optional.Of[codec.List[CodecZenohReplyExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]]
	body codec_zenoh_push_body.CodecZenohPushBodySource
}

// MakeCodecZenohReply returns the CodecZenohReply that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohReply, or as its zero value, and is read through its methods.
func MakeCodecZenohReply(header uint8, consolidation optional.Of[uint8], extensions optional.Of[codec.List[CodecZenohReplyExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]], body codec_zenoh_push_body.CodecZenohPushBodySource) CodecZenohReply {
	return CodecZenohReply{header: header, consolidation: consolidation, extensions: extensions, body: body}
}

// Header returns the element the value presents as Header.
func (s *CodecZenohReply) Header() uint8 {
	return s.header
}

// Consolidation returns the element the value presents as Consolidation.
func (s *CodecZenohReply) Consolidation() optional.Of[uint8] {
	return s.consolidation
}

// Extensions returns the element the value presents as Extensions.
func (s *CodecZenohReply) Extensions() optional.Of[codec.List[CodecZenohReplyExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]] {
	return s.extensions
}

// Body returns the element the value presents as Body.
func (s *CodecZenohReply) Body() codec_zenoh_push_body.CodecZenohPushBodySource {
	if s.body == nil {
		return &codec_zenoh_push_body.CodecZenohPushBody{}
	}
	return s.body
}

// CodecZenohReplyOverrides holds the one-bit flags of CodecZenohReply a parent may
// set in place of the ones a value presents, such as the flag a chain whose
// entries are CodecZenohReply ends them on, which the chain derives from where
// each entry stands. A flag it leaves absent is the one the value presents.
type CodecZenohReplyOverrides struct {
	// HeaderX is the `X` flag of `header`, where a parent sets it.
	HeaderX optional.Of[bool]
}

// EncodeSource writes any source of CodecZenohReply into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohReplySource, w codec.SceSink) error {
	return EncodeSourceOverriding(src, w, CodecZenohReplyOverrides{})
}

// EncodeSourceOverriding is EncodeSource, each flag overrides sets written in
// place of the one src presents.
func EncodeSourceOverriding(src CodecZenohReplySource, w codec.SceSink, overrides CodecZenohReplyOverrides) error {
	_cHeader := src.Header()
	_cHeader &^= 0x20
	if src.Consolidation().IsPresent() {
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
	if src.Consolidation().IsPresent() {
		if err := w.WriteBytes([]byte{byte(src.Consolidation().Proven())}); err != nil {
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
	if err := codec_zenoh_push_body.EncodeSource(src.Body(), w); err != nil {
		return err
	}
	return nil
}

// NewCodecZenohReply returns a CodecZenohReply initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecZenohReply().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecZenohReply{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecZenohReply() *CodecZenohReply {
	return &CodecZenohReply{
		header: uint8(0x04),
	}
}

// DecodeCodecZenohReply decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohReply(cursor *codec.SceCursor) (*CodecZenohReply, error) {
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
	var Consolidation optional.Of[uint8]
	if (Header & 0x20) != 0 {
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		_v := raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
		Consolidation = optional.Some(_v)
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
	var Body codec_zenoh_push_body.CodecZenohPushBody
	{
		_emb, err := codec_zenoh_push_body.DecodeCodecZenohPushBody(cursor)
		if err != nil {
			return nil, err
		}
		Body = *_emb
	}
	_lExtensions, err := codec.MakeOptionalList[CodecZenohReplyExtensionsBound](codec.OptionalSourcesOf(Extensions, func(e *codec_zenoh_ext_entry.CodecZenohExtEntry) codec_zenoh_ext_entry.CodecZenohExtEntrySource { return e }))
	if err != nil {
		return nil, err
	}
	return &CodecZenohReply{
		header: Header,
		consolidation: Consolidation,
		extensions: _lExtensions,
		body: &Body,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohReply) Mid() uint8 {
	return uint8((s.header >> 0) & 0x1F)
}

func (s *CodecZenohReply) SetMid(v uint8) {
	const _shiftedMask uint8 = 0x1F << 0
	_val := (uint8(v) & 0x1F) << 0
	s.header = (s.header &^ _shiftedMask) | _val
}

func (s *CodecZenohReply) C() bool {
	return (s.header & 0x20) != 0
}

func (s *CodecZenohReply) SetC(v bool) {
	if v {
		s.header |= 0x20
	} else {
		s.header &^= 0x20
	}
}

func (s *CodecZenohReply) X() bool {
	return (s.header & 0x40) != 0
}

func (s *CodecZenohReply) SetX(v bool) {
	if v {
		s.header |= 0x40
	} else {
		s.header &^= 0x40
	}
}

func (s *CodecZenohReply) Z() bool {
	return (s.header & 0x80) != 0
}

func (s *CodecZenohReply) SetZ(v bool) {
	if v {
		s.header |= 0x80
	} else {
		s.header &^= 0x80
	}
}

// Encode writes the CodecZenohReply into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohReply) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohReply) EncodeToBytes() []byte {
	_dst := make([]byte, 0)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
