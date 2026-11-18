// SCE-MAP: codec_zenoh_ext_envelope:35 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_ext_envelope

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
	"example.com/sce-forge/codec_zenoh_ext_entry"
)

// CodecZenohExtEnvelopeExtensionsBound is the bound of the extensions element of CodecZenohExtEnvelope: 1 or more entries, the protocol stating no most
// (docs/adr/0065 §6 R2). A List of it is made only within it.
type CodecZenohExtEnvelopeExtensionsBound struct{}

// Least is the fewest entries the element holds.
func (CodecZenohExtEnvelopeExtensionsBound) Least() int { return 1 }

// Most is the most entries the element holds.
func (CodecZenohExtEnvelopeExtensionsBound) Most() int { return codec.Unbounded }

// CodecZenohExtEnvelopeSource is what the encode of CodecZenohExtEnvelope reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohExtEnvelope is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohExtEnvelopeSource interface {
	HeaderFlags() uint8
	Extensions() codec.List[CodecZenohExtEnvelopeExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]
}

// CodecZenohExtEnvelope represents the codec frame layout.
type CodecZenohExtEnvelope struct {
	headerFlags uint8
	extensions codec.List[CodecZenohExtEnvelopeExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]
}

// MakeCodecZenohExtEnvelope returns the CodecZenohExtEnvelope that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohExtEnvelope, or as its zero value, and is read through its methods.
func MakeCodecZenohExtEnvelope(headerFlags uint8, extensions codec.List[CodecZenohExtEnvelopeExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource]) CodecZenohExtEnvelope {
	return CodecZenohExtEnvelope{headerFlags: headerFlags, extensions: extensions}
}

// HeaderFlags returns the element the value presents as HeaderFlags.
func (s *CodecZenohExtEnvelope) HeaderFlags() uint8 {
	return s.headerFlags
}

// Extensions returns the element the value presents as Extensions.
func (s *CodecZenohExtEnvelope) Extensions() codec.List[CodecZenohExtEnvelopeExtensionsBound, codec_zenoh_ext_entry.CodecZenohExtEntrySource] {
	return s.extensions
}

// EncodeSource writes any source of CodecZenohExtEnvelope into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohExtEnvelopeSource, w codec.SceSink) error {
	if err := w.WriteBytes([]byte{byte(src.HeaderFlags())}); err != nil {
		return err
	}
	{
		_leftExtensions := src.Extensions().Len()
		for _e := range src.Extensions().All() {
			_leftExtensions--
			if err := codec_zenoh_ext_entry.EncodeSourceOverriding(_e, w, codec_zenoh_ext_entry.CodecZenohExtEntryOverrides{HeaderZ: optional.Some(_leftExtensions != 0)}); err != nil {
				return err
			}
		}
	}
	return nil
}

// DecodeCodecZenohExtEnvelope decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohExtEnvelope(cursor *codec.SceCursor) (*CodecZenohExtEnvelope, error) {
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
	Extensions := make([]codec_zenoh_ext_entry.CodecZenohExtEntry, 0)
	_more := false
	for {
		_elem, err := codec_zenoh_ext_entry.DecodeCodecZenohExtEntry(cursor)
		if err != nil {
			return nil, err
		}
		_more = _elem.Z()
		Extensions = append(Extensions, *_elem)
		if !_more {
			break
		}
	}
	if _more && cursor.Remaining() == 0 {
		return nil, codec.ErrNeedMoreBytes
	}
	_lExtensions, err := codec.MakeList[CodecZenohExtEnvelopeExtensionsBound](codec.SourcesOf(Extensions, func(e *codec_zenoh_ext_entry.CodecZenohExtEntry) codec_zenoh_ext_entry.CodecZenohExtEntrySource { return e }))
	if err != nil {
		return nil, err
	}
	return &CodecZenohExtEnvelope{
		headerFlags: HeaderFlags,
		extensions: _lExtensions,
	}, nil
}

// Encode writes the CodecZenohExtEnvelope into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohExtEnvelope) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohExtEnvelope) EncodeToBytes() []byte {
	_dst := make([]byte, 0)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
