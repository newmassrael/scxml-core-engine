// SCE-MAP: codec_until_eof_basic:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_until_eof_basic

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_repeat_elem"
)

// CodecUntilEofBasicMsgsBound is the bound of the msgs element of CodecUntilEofBasic: 0 to 64 entries
// (docs/adr/0065 §6 R2). A List of it is made only within it.
type CodecUntilEofBasicMsgsBound struct{}

// Least is the fewest entries the element holds.
func (CodecUntilEofBasicMsgsBound) Least() int { return 0 }

// Most is the most entries the element holds.
func (CodecUntilEofBasicMsgsBound) Most() int { return 64 }

// CodecUntilEofBasicSource is what the encode of CodecUntilEofBasic reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecUntilEofBasic is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecUntilEofBasicSource interface {
	Msgs() codec.List[CodecUntilEofBasicMsgsBound, codec_repeat_elem.CodecRepeatElemSource]
}

// CodecUntilEofBasic represents the codec frame layout.
type CodecUntilEofBasic struct {
	msgs codec.List[CodecUntilEofBasicMsgsBound, codec_repeat_elem.CodecRepeatElemSource]
}

// MakeCodecUntilEofBasic returns the CodecUntilEofBasic that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecUntilEofBasic, or as its zero value, and is read through its methods.
func MakeCodecUntilEofBasic(msgs codec.List[CodecUntilEofBasicMsgsBound, codec_repeat_elem.CodecRepeatElemSource]) CodecUntilEofBasic {
	return CodecUntilEofBasic{msgs: msgs}
}

// Msgs returns the element the value presents as Msgs.
func (s *CodecUntilEofBasic) Msgs() codec.List[CodecUntilEofBasicMsgsBound, codec_repeat_elem.CodecRepeatElemSource] {
	return s.msgs
}

// EncodeSource writes any source of CodecUntilEofBasic into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecUntilEofBasicSource, w codec.SceSink) error {
	for _e := range src.Msgs().All() {
		if err := codec_repeat_elem.EncodeSource(_e, w); err != nil {
			return err
		}
	}
	return nil
}

// DecodeCodecUntilEofBasic decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecUntilEofBasic(cursor *codec.SceCursor) (*CodecUntilEofBasic, error) {
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
	Msgs := make([]codec_repeat_elem.CodecRepeatElem, 0)
	for cursor.Remaining() > 0 {
		if len(Msgs) > 63 {
			return nil, codec.ErrTooManyElements
		}
		_elem, err := codec_repeat_elem.DecodeCodecRepeatElem(cursor)
		if err != nil {
			return nil, err
		}
		Msgs = append(Msgs, *_elem)
	}
	_lMsgs, err := codec.MakeList[CodecUntilEofBasicMsgsBound](codec.SourcesOf(Msgs, func(e *codec_repeat_elem.CodecRepeatElem) codec_repeat_elem.CodecRepeatElemSource { return e }))
	if err != nil {
		return nil, err
	}
	return &CodecUntilEofBasic{
		msgs: _lMsgs,
	}, nil
}

// Encode writes the CodecUntilEofBasic into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecUntilEofBasic) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecUntilEofBasic) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 128)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
