// SCE-MAP: codec_zenoh_response_final:41 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_response_final

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
	"example.com/sce-forge/codec_zenoh_ext_entry"
)

// CodecZenohResponseFinalExtensionsBound is the bound of the extensions element of CodecZenohResponseFinal: 1 or more entries, the protocol stating no most
// (docs/adr/0065 §6 R2). A List of it is made only within it.
type CodecZenohResponseFinalExtensionsBound struct{}

// Least is the fewest entries the element holds.
func (CodecZenohResponseFinalExtensionsBound) Least() int { return 1 }

// Most is the most entries the element holds.
func (CodecZenohResponseFinalExtensionsBound) Most() int { return codec.Unbounded }

// CodecZenohResponseFinalSource is what the encode of CodecZenohResponseFinal reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohResponseFinal is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohResponseFinalSource interface {
	Header() uint8
	RequestId() uint64
	Extensions() optional.Of[codec.List[CodecZenohResponseFinalExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]]
}

// CodecZenohResponseFinal represents the codec frame layout.
type CodecZenohResponseFinal struct {
	header uint8
	requestId uint64
	extensions optional.Of[codec.List[CodecZenohResponseFinalExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]]
}

// MakeCodecZenohResponseFinal returns the CodecZenohResponseFinal that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohResponseFinal, or as its zero value, and is read through its methods.
func MakeCodecZenohResponseFinal(header uint8, requestId uint64, extensions optional.Of[codec.List[CodecZenohResponseFinalExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]]) CodecZenohResponseFinal {
	return CodecZenohResponseFinal{header: header, requestId: requestId, extensions: extensions}
}

// Header returns the element the value presents as Header.
func (s *CodecZenohResponseFinal) Header() uint8 {
	return s.header
}

// RequestId returns the element the value presents as RequestId.
func (s *CodecZenohResponseFinal) RequestId() uint64 {
	return s.requestId
}

// Extensions returns the element the value presents as Extensions.
func (s *CodecZenohResponseFinal) Extensions() optional.Of[codec.List[CodecZenohResponseFinalExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]] {
	return s.extensions
}

// EncodeSource writes any source of CodecZenohResponseFinal into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohResponseFinalSource, w codec.SceSink) error {
	_cHeader := src.Header()
	_cHeader &^= 0x80
	if src.Extensions().IsPresent() {
		_cHeader |= 0x80
	}
	if err := w.WriteBytes([]byte{byte(_cHeader)}); err != nil {
		return err
	}
	if err := codec.WriteVLEU64(w, uint64(src.RequestId())); err != nil {
		return err
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

// NewCodecZenohResponseFinal returns a CodecZenohResponseFinal initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecZenohResponseFinal().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecZenohResponseFinal{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecZenohResponseFinal() *CodecZenohResponseFinal {
	return &CodecZenohResponseFinal{
		header: uint8(0x1a),
	}
}

// DecodeCodecZenohResponseFinal decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohResponseFinal(cursor *codec.SceCursor) (*CodecZenohResponseFinal, error) {
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
	RequestId, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
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
	_lExtensions, err := codec.MakeOptionalList[CodecZenohResponseFinalExtensionsBound](codec.OptionalSourcesOf(Extensions, func(e *codec_zenoh_ext_entry.CodecZenohExtEntry) codec_zenoh_ext_entry.CodecZenohExtEntrySource { return e }))
	if err != nil {
		return nil, err
	}
	return &CodecZenohResponseFinal{
		header: Header,
		requestId: RequestId,
		extensions: _lExtensions,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohResponseFinal) Mid() uint8 {
	return uint8((s.header >> 0) & 0x1F)
}

func (s *CodecZenohResponseFinal) SetMid(v uint8) {
	const _shiftedMask uint8 = 0x1F << 0
	_val := (uint8(v) & 0x1F) << 0
	s.header = (s.header &^ _shiftedMask) | _val
}

func (s *CodecZenohResponseFinal) Z() bool {
	return (s.header & 0x80) != 0
}

func (s *CodecZenohResponseFinal) SetZ(v bool) {
	if v {
		s.header |= 0x80
	} else {
		s.header &^= 0x80
	}
}

// Encode writes the CodecZenohResponseFinal into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohResponseFinal) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohResponseFinal) EncodeToBytes() []byte {
	_dst := make([]byte, 0)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
