// SCE-MAP: codec_repeat_unconditional_count:34 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_repeat_unconditional_count

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_repeat_elem"
)

// CodecRepeatUnconditionalCount represents the codec frame layout.
type CodecRepeatUnconditionalCount struct {
	Options uint8
	LinksLen uint8
	Links []codec_repeat_elem.CodecRepeatElem
	Weights []codec_repeat_elem.CodecRepeatElem
}

// DecodeCodecRepeatUnconditionalCount decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecRepeatUnconditionalCount(cursor *codec.SceCursor) (*CodecRepeatUnconditionalCount, error) {
	// Streaming cursor decode (SSOT selection: `needs_streaming`).
	// The positional `raw[byte_off]` path is valid only when every
	// field's absolute offset is fixed at codegen time; this branch
	// handles every codec where it is not — present-if-gated fields
	// (runtime presence; `*T` / nil `[]byte`), VLE / repeat / TLV-chain /
	// embed fields (runtime width), string fields (UTF-8 decode), and a
	// fixed field after a variable-length payload (offset depends on the
	// payload length). Each field reads its own bytes from the cursor and
	// advances past what it consumed. Per-field `is_repeat` /
	// `is_tlv_chain` / `is_embed` route to their dedicated helpers; every
	// other field flows through `present_if_decode_stmt`.
	var Options uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Options = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var LinksLen uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		LinksLen = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	Links := make([]codec_repeat_elem.CodecRepeatElem, 0, LinksLen)
	for _i := 0; _i < int(LinksLen); _i++ {
		_elem, err := codec_repeat_elem.DecodeCodecRepeatElem(cursor)
		if err != nil {
			return nil, err
		}
		Links = append(Links, *_elem)
	}
	var Weights []codec_repeat_elem.CodecRepeatElem
	if (Options & 0x01) != 0 {
		_n := LinksLen
		Weights = make([]codec_repeat_elem.CodecRepeatElem, 0, _n)
		for _i := 0; _i < int(_n); _i++ {
			_elem, err := codec_repeat_elem.DecodeCodecRepeatElem(cursor)
			if err != nil {
				return nil, err
			}
			Weights = append(Weights, *_elem)
		}
	}
	return &CodecRepeatUnconditionalCount{
		Options: Options,
		LinksLen: LinksLen,
		Links: Links,
		Weights: Weights,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecRepeatUnconditionalCount) H() bool {
	return (s.Options & 0x01) != 0
}

func (s *CodecRepeatUnconditionalCount) SetH(v bool) {
	if v {
		s.Options |= 0x01
	} else {
		s.Options &^= 0x01
	}
}

// Encode writes the CodecRepeatUnconditionalCount into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecRepeatUnconditionalCount) Encode(w codec.SceSink) error {
	// Streaming cursor encode (SSOT selection: `needs_streaming`).
	// Mirrors the streaming decode: every field appends its own bytes in
	// declaration order through the per-field encode blocks, so a gated
	// field skips its append when absent, and a fixed field after a
	// variable-length payload lands after the payload (the positional path
	// appends variable fields last, placing it ahead on the wire).
	// Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
	// dedicated helpers; everything else uses `present_if_encode_block`.
	if err := w.WriteBytes([]byte{ s.Options }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ s.LinksLen }); err != nil {
		return err
	}
	for _i := range s.Links {
		if err := s.Links[_i].Encode(w); err != nil {
			return err
		}
	}
	if s.Weights != nil {
		for _i := range s.Weights {
			if err := s.Weights[_i].Encode(w); err != nil {
				return err
			}
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecRepeatUnconditionalCount) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 258)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
