// SCE-MAP: codec_embed_basic:43 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_embed_basic

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_locator"
)

// CodecEmbedBasicSource is what the encode of CodecEmbedBasic reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecEmbedBasic is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecEmbedBasicSource interface {
	Tag() uint8
	Locator() codec_zenoh_locator.CodecZenohLocatorSource
}

// CodecEmbedBasic represents the codec frame layout.
type CodecEmbedBasic struct {
	tag uint8
	locator codec_zenoh_locator.CodecZenohLocatorSource
}

// MakeCodecEmbedBasic returns the CodecEmbedBasic that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecEmbedBasic, or as its zero value, and is read through its methods.
func MakeCodecEmbedBasic(tag uint8, locator codec_zenoh_locator.CodecZenohLocatorSource) CodecEmbedBasic {
	return CodecEmbedBasic{tag: tag, locator: locator}
}

// Tag returns the element the value presents as Tag.
func (s *CodecEmbedBasic) Tag() uint8 {
	return s.tag
}

// Locator returns the element the value presents as Locator.
func (s *CodecEmbedBasic) Locator() codec_zenoh_locator.CodecZenohLocatorSource {
	if s.locator == nil {
		return &codec_zenoh_locator.CodecZenohLocator{}
	}
	return s.locator
}

// EncodeSource writes any source of CodecEmbedBasic into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecEmbedBasicSource, w codec.SceSink) error {
	if err := w.WriteBytes([]byte{byte(src.Tag())}); err != nil {
		return err
	}
	if err := codec_zenoh_locator.EncodeSource(src.Locator(), w); err != nil {
		return err
	}
	return nil
}

// DecodeCodecEmbedBasic decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecEmbedBasic(cursor *codec.SceCursor) (*CodecEmbedBasic, error) {
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
	var Tag uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Tag = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var Locator codec_zenoh_locator.CodecZenohLocator
	{
		_emb, err := codec_zenoh_locator.DecodeCodecZenohLocator(cursor)
		if err != nil {
			return nil, err
		}
		Locator = *_emb
	}
	return &CodecEmbedBasic{
		tag: Tag,
		locator: &Locator,
	}, nil
}

// Encode writes the CodecEmbedBasic into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecEmbedBasic) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecEmbedBasic) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 265)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
