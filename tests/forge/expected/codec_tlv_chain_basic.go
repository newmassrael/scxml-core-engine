// SCE-MAP: codec_tlv_chain_basic:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_tlv_chain_basic

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_tlv_entry"
)

// CodecTlvChainBasicExtensionsBound is the bound of the extensions element of CodecTlvChainBasic: 0 to 8 entries
// (docs/adr/0065 §6 R2). A List of it is made only within it.
type CodecTlvChainBasicExtensionsBound struct{}

// Least is the fewest entries the element holds.
func (CodecTlvChainBasicExtensionsBound) Least() int { return 0 }

// Most is the most entries the element holds.
func (CodecTlvChainBasicExtensionsBound) Most() int { return 8 }

// CodecTlvChainBasicSource is what the encode of CodecTlvChainBasic reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecTlvChainBasic is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecTlvChainBasicSource interface {
	HeaderFlags() uint8
	Extensions() codec.List[CodecTlvChainBasicExtensionsBound, codec_tlv_entry.CodecTlvEntrySource]
}

// CodecTlvChainBasic represents the codec frame layout.
type CodecTlvChainBasic struct {
	headerFlags uint8
	extensions codec.List[CodecTlvChainBasicExtensionsBound, codec_tlv_entry.CodecTlvEntrySource]
}

// MakeCodecTlvChainBasic returns the CodecTlvChainBasic that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecTlvChainBasic, or as its zero value, and is read through its methods.
func MakeCodecTlvChainBasic(headerFlags uint8, extensions codec.List[CodecTlvChainBasicExtensionsBound, codec_tlv_entry.CodecTlvEntrySource]) CodecTlvChainBasic {
	return CodecTlvChainBasic{headerFlags: headerFlags, extensions: extensions}
}

// HeaderFlags returns the element the value presents as HeaderFlags.
func (s *CodecTlvChainBasic) HeaderFlags() uint8 {
	return s.headerFlags
}

// Extensions returns the element the value presents as Extensions.
func (s *CodecTlvChainBasic) Extensions() codec.List[CodecTlvChainBasicExtensionsBound, codec_tlv_entry.CodecTlvEntrySource] {
	return s.extensions
}

// EncodeSource writes any source of CodecTlvChainBasic into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecTlvChainBasicSource, w codec.SceSink) error {
	if err := w.WriteBytes([]byte{byte(src.HeaderFlags())}); err != nil {
		return err
	}
	for _e := range src.Extensions().All() {
		if err := codec_tlv_entry.EncodeSource(_e, w); err != nil {
			return err
		}
	}
	return nil
}

// DecodeCodecTlvChainBasic decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecTlvChainBasic(cursor *codec.SceCursor) (*CodecTlvChainBasic, error) {
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
	var HeaderFlags uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		HeaderFlags = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	Extensions := make([]codec_tlv_entry.CodecTlvEntry, 0, 8)
	for _i := 0; _i < int(8); _i++ {
		if cursor.Remaining() == 0 {
			break
		}
		_elem, err := codec_tlv_entry.DecodeCodecTlvEntry(cursor)
		if err != nil {
			return nil, err
		}
		Extensions = append(Extensions, *_elem)
	}
	if cursor.Remaining() > 0 {
		return nil, codec.ErrTooManyElements
	}
	_lExtensions, err := codec.MakeList[CodecTlvChainBasicExtensionsBound](codec.SourcesOf(Extensions, func(e *codec_tlv_entry.CodecTlvEntry) codec_tlv_entry.CodecTlvEntrySource { return e }))
	if err != nil {
		return nil, err
	}
	return &CodecTlvChainBasic{
		headerFlags: HeaderFlags,
		extensions: _lExtensions,
	}, nil
}

// Encode writes the CodecTlvChainBasic into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecTlvChainBasic) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecTlvChainBasic) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 273)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
