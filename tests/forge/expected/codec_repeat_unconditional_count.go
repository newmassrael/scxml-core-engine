// SCE-MAP: codec_repeat_unconditional_count:34 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_repeat_unconditional_count

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
	"example.com/sce-forge/codec_repeat_elem"
)

// CodecRepeatUnconditionalCountLinksBound is the bound of the links element of CodecRepeatUnconditionalCount: 0 to 64 entries
// (docs/adr/0065 §6 R2). A List of it is made only within it.
type CodecRepeatUnconditionalCountLinksBound struct{}

// Least is the fewest entries the element holds.
func (CodecRepeatUnconditionalCountLinksBound) Least() int { return 0 }

// Most is the most entries the element holds.
func (CodecRepeatUnconditionalCountLinksBound) Most() int { return 64 }

// CodecRepeatUnconditionalCountWeightsBound is the bound of the weights element of CodecRepeatUnconditionalCount: 0 to 64 entries
// (docs/adr/0065 §6 R2). A List of it is made only within it.
type CodecRepeatUnconditionalCountWeightsBound struct{}

// Least is the fewest entries the element holds.
func (CodecRepeatUnconditionalCountWeightsBound) Least() int { return 0 }

// Most is the most entries the element holds.
func (CodecRepeatUnconditionalCountWeightsBound) Most() int { return 64 }

// CodecRepeatUnconditionalCountOptionsHH is the OptionsH group of CodecRepeatUnconditionalCount where `options.H` holds (docs/adr/0065 §6 R4): the
// members present then.
type CodecRepeatUnconditionalCountOptionsHH struct {
	linksWeights codec.Aligned[codec.List[CodecRepeatUnconditionalCountLinksBound, codec_repeat_elem.CodecRepeatElemSource], codec.List[CodecRepeatUnconditionalCountWeightsBound, codec_repeat_elem.CodecRepeatElemSource]]
}

// MakeCodecRepeatUnconditionalCountOptionsHH returns the arm that holds each member it is handed.
func MakeCodecRepeatUnconditionalCountOptionsHH(linksWeights codec.Aligned[codec.List[CodecRepeatUnconditionalCountLinksBound, codec_repeat_elem.CodecRepeatElemSource], codec.List[CodecRepeatUnconditionalCountWeightsBound, codec_repeat_elem.CodecRepeatElemSource]]) CodecRepeatUnconditionalCountOptionsHH {
	return CodecRepeatUnconditionalCountOptionsHH{linksWeights: linksWeights}
}

// LinksWeights returns the member the arm presents as LinksWeights.
func (a CodecRepeatUnconditionalCountOptionsHH) LinksWeights() codec.Aligned[codec.List[CodecRepeatUnconditionalCountLinksBound, codec_repeat_elem.CodecRepeatElemSource], codec.List[CodecRepeatUnconditionalCountWeightsBound, codec_repeat_elem.CodecRepeatElemSource]] {
	return a.linksWeights
}

func (CodecRepeatUnconditionalCountOptionsHH) isCodecRepeatUnconditionalCountOptionsH() {}

// CodecRepeatUnconditionalCountOptionsHNotH is the OptionsH group of CodecRepeatUnconditionalCount where `options.H` does not hold (docs/adr/0065 §6 R4): the
// members present then.
type CodecRepeatUnconditionalCountOptionsHNotH struct {
	links codec.List[CodecRepeatUnconditionalCountLinksBound, codec_repeat_elem.CodecRepeatElemSource]
}

// MakeCodecRepeatUnconditionalCountOptionsHNotH returns the arm that holds each member it is handed.
func MakeCodecRepeatUnconditionalCountOptionsHNotH(links codec.List[CodecRepeatUnconditionalCountLinksBound, codec_repeat_elem.CodecRepeatElemSource]) CodecRepeatUnconditionalCountOptionsHNotH {
	return CodecRepeatUnconditionalCountOptionsHNotH{links: links}
}

// Links returns the member the arm presents as Links.
func (a CodecRepeatUnconditionalCountOptionsHNotH) Links() codec.List[CodecRepeatUnconditionalCountLinksBound, codec_repeat_elem.CodecRepeatElemSource] {
	return a.links
}

func (CodecRepeatUnconditionalCountOptionsHNotH) isCodecRepeatUnconditionalCountOptionsH() {}

// CodecRepeatUnconditionalCountOptionsH is the OptionsH group of CodecRepeatUnconditionalCount: the arm the value holds, each the
// members present while the gates hold its values (docs/adr/0065 §6 R4),
// one of CodecRepeatUnconditionalCountOptionsHH, CodecRepeatUnconditionalCountOptionsHNotH, which only this package declares.
type CodecRepeatUnconditionalCountOptionsH interface {
	isCodecRepeatUnconditionalCountOptionsH()
}

// CodecRepeatUnconditionalCountSource is what the encode of CodecRepeatUnconditionalCount reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecRepeatUnconditionalCount is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecRepeatUnconditionalCountSource interface {
	Options() uint8
	OptionsH() CodecRepeatUnconditionalCountOptionsH
}

// CodecRepeatUnconditionalCount represents the codec frame layout.
type CodecRepeatUnconditionalCount struct {
	options uint8
	optionsH CodecRepeatUnconditionalCountOptionsH
}

// MakeCodecRepeatUnconditionalCount returns the CodecRepeatUnconditionalCount that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecRepeatUnconditionalCount, or as its zero value, and is read through its methods.
func MakeCodecRepeatUnconditionalCount(options uint8, optionsH CodecRepeatUnconditionalCountOptionsH) CodecRepeatUnconditionalCount {
	return CodecRepeatUnconditionalCount{options: options, optionsH: optionsH}
}

// Options returns the element the value presents as Options.
func (s *CodecRepeatUnconditionalCount) Options() uint8 {
	return s.options
}

// OptionsH returns the element the value presents as OptionsH.
func (s *CodecRepeatUnconditionalCount) OptionsH() CodecRepeatUnconditionalCountOptionsH {
	if s.optionsH == nil {
		return CodecRepeatUnconditionalCountOptionsHH{}
	}
	return s.optionsH
}

// WireLinksLen returns the links_len the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecRepeatUnconditionalCount) WireLinksLen() uint8 {
	_rOptionsH := s.OptionsH()
	var _mLinks optional.Of[codec.List[CodecRepeatUnconditionalCountLinksBound, codec_repeat_elem.CodecRepeatElemSource]]
	switch _g := _rOptionsH.(type) {
	case CodecRepeatUnconditionalCountOptionsHH:
		_mLinks = optional.Some(_g.LinksWeights().First())
	case CodecRepeatUnconditionalCountOptionsHNotH:
		_mLinks = optional.Some(_g.Links())
	}
	return uint8(_mLinks.Proven().Len())
}

// EncodeSource writes any source of CodecRepeatUnconditionalCount into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives links_len from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecRepeatUnconditionalCountSource, w codec.SceSink) error {
	_rOptionsH := src.OptionsH()
	var _mLinks optional.Of[codec.List[CodecRepeatUnconditionalCountLinksBound, codec_repeat_elem.CodecRepeatElemSource]]
	var _mWeights optional.Of[codec.List[CodecRepeatUnconditionalCountWeightsBound, codec_repeat_elem.CodecRepeatElemSource]]
	switch _g := _rOptionsH.(type) {
	case CodecRepeatUnconditionalCountOptionsHH:
		_mLinks = optional.Some(_g.LinksWeights().First())
		_mWeights = optional.Some(_g.LinksWeights().Second())
	case CodecRepeatUnconditionalCountOptionsHNotH:
		_mLinks = optional.Some(_g.Links())
	}
	_dLinksLen := uint8(_mLinks.Proven().Len())
	_cOptions := src.Options()
	_cOptions &^= 0x1
	if _mWeights.IsPresent() {
		_cOptions |= 0x1
	}
	if err := w.WriteBytes([]byte{byte(_cOptions)}); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{byte(_dLinksLen)}); err != nil {
		return err
	}
	if _mLinks.IsPresent() {
		for _e := range _mLinks.Proven().All() {
			if err := codec_repeat_elem.EncodeSource(_e, w); err != nil {
				return err
			}
		}
	}
	if _mWeights.IsPresent() {
		for _e := range _mWeights.Proven().All() {
			if err := codec_repeat_elem.EncodeSource(_e, w); err != nil {
				return err
			}
		}
	}
	return nil
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
	// (runtime presence, held by the runtime's optional type), VLE / repeat / TLV-chain /
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
	if LinksLen > 64 {
		return nil, codec.ErrTooManyElements
	}
	Links := make([]codec_repeat_elem.CodecRepeatElem, 0, cursor.EntriesBacked(uint64(LinksLen)))
	for _i := 0; _i < int(LinksLen); _i++ {
		_elem, err := codec_repeat_elem.DecodeCodecRepeatElem(cursor)
		if err != nil {
			return nil, err
		}
		Links = append(Links, *_elem)
	}
	var Weights optional.Of[[]codec_repeat_elem.CodecRepeatElem]
	if (Options & 0x01) != 0 {
		_n := LinksLen
		if _n > 64 {
			return nil, codec.ErrTooManyElements
		}
		_list := make([]codec_repeat_elem.CodecRepeatElem, 0, cursor.EntriesBacked(uint64(_n)))
		for _i := 0; _i < int(_n); _i++ {
			_elem, err := codec_repeat_elem.DecodeCodecRepeatElem(cursor)
			if err != nil {
				return nil, err
			}
			_list = append(_list, *_elem)
		}
		Weights = optional.Some(_list)
	}
	_lLinks, err := codec.MakeList[CodecRepeatUnconditionalCountLinksBound](codec.SourcesOf(Links, func(e *codec_repeat_elem.CodecRepeatElem) codec_repeat_elem.CodecRepeatElemSource { return e }))
	if err != nil {
		return nil, err
	}
	_lWeights, err := codec.MakeOptionalList[CodecRepeatUnconditionalCountWeightsBound](codec.OptionalSourcesOf(Weights, func(e *codec_repeat_elem.CodecRepeatElem) codec_repeat_elem.CodecRepeatElemSource { return e }))
	if err != nil {
		return nil, err
	}
	var _gOptionsH CodecRepeatUnconditionalCountOptionsH
	if ((Options >> 0) & 1) != 0 {
		_aLinksWeights, err := codec.MakeAligned(_lLinks, _lWeights.Proven())
		if err != nil {
			return nil, err
		}
		_gOptionsH = MakeCodecRepeatUnconditionalCountOptionsHH(_aLinksWeights)
	} else {
		_gOptionsH = MakeCodecRepeatUnconditionalCountOptionsHNotH(_lLinks)
	}
	return &CodecRepeatUnconditionalCount{
		options: Options,
		optionsH: _gOptionsH,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecRepeatUnconditionalCount) H() bool {
	return (s.options & 0x01) != 0
}

func (s *CodecRepeatUnconditionalCount) SetH(v bool) {
	if v {
		s.options |= 0x01
	} else {
		s.options &^= 0x01
	}
}

// Encode writes the CodecRepeatUnconditionalCount into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecRepeatUnconditionalCount) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
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
