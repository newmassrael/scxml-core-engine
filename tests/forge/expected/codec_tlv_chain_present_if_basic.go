// SCE-MAP: codec_tlv_chain_present_if_basic:37 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_tlv_chain_present_if_basic

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
	"example.com/sce-forge/codec_tlv_entry"
)

// CodecTlvChainPresentIfBasicEntriesBound is the bound of the entries element of CodecTlvChainPresentIfBasic: 0 to 4 entries
// (docs/adr/0065 §6 R2). A List of it is made only within it.
type CodecTlvChainPresentIfBasicEntriesBound struct{}

// Least is the fewest entries the element holds.
func (CodecTlvChainPresentIfBasicEntriesBound) Least() int { return 0 }

// Most is the most entries the element holds.
func (CodecTlvChainPresentIfBasicEntriesBound) Most() int { return 4 }

// CodecTlvChainPresentIfBasicSource is what the encode of CodecTlvChainPresentIfBasic reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecTlvChainPresentIfBasic is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecTlvChainPresentIfBasicSource interface {
	Carrier() uint8
	Entries() optional.Of[codec.List[CodecTlvChainPresentIfBasicEntriesBound, codec_tlv_entry.CodecTlvEntrySource]]
}

// CodecTlvChainPresentIfBasic represents the codec frame layout.
type CodecTlvChainPresentIfBasic struct {
	carrier uint8
	entries optional.Of[codec.List[CodecTlvChainPresentIfBasicEntriesBound, codec_tlv_entry.CodecTlvEntrySource]]
}

// MakeCodecTlvChainPresentIfBasic returns the CodecTlvChainPresentIfBasic that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecTlvChainPresentIfBasic, or as its zero value, and is read through its methods.
func MakeCodecTlvChainPresentIfBasic(carrier uint8, entries optional.Of[codec.List[CodecTlvChainPresentIfBasicEntriesBound, codec_tlv_entry.CodecTlvEntrySource]]) CodecTlvChainPresentIfBasic {
	return CodecTlvChainPresentIfBasic{carrier: carrier, entries: entries}
}

// Carrier returns the element the value presents as Carrier.
func (s *CodecTlvChainPresentIfBasic) Carrier() uint8 {
	return s.carrier
}

// Entries returns the element the value presents as Entries.
func (s *CodecTlvChainPresentIfBasic) Entries() optional.Of[codec.List[CodecTlvChainPresentIfBasicEntriesBound, codec_tlv_entry.CodecTlvEntrySource]] {
	return s.entries
}

// EncodeSource writes any source of CodecTlvChainPresentIfBasic into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecTlvChainPresentIfBasicSource, w codec.SceSink) error {
	_cCarrier := src.Carrier()
	_cCarrier &^= 0x1
	if src.Entries().IsPresent() {
		_cCarrier |= 0x1
	}
	if err := w.WriteBytes([]byte{byte(_cCarrier)}); err != nil {
		return err
	}
	if src.Entries().IsPresent() {
		for _e := range src.Entries().Proven().All() {
			if err := codec_tlv_entry.EncodeSource(_e, w); err != nil {
				return err
			}
		}
	}
	return nil
}

// DecodeCodecTlvChainPresentIfBasic decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecTlvChainPresentIfBasic(cursor *codec.SceCursor) (*CodecTlvChainPresentIfBasic, error) {
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
	var Carrier uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Carrier = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var Entries optional.Of[[]codec_tlv_entry.CodecTlvEntry]
	if (Carrier & 0x01) != 0 {
		_list := make([]codec_tlv_entry.CodecTlvEntry, 0, 4)
		for _i := 0; _i < int(4); _i++ {
			if cursor.Remaining() == 0 {
				break
			}
			_elem, err := codec_tlv_entry.DecodeCodecTlvEntry(cursor)
			if err != nil {
				return nil, err
			}
			_list = append(_list, *_elem)
		}
		if cursor.Remaining() > 0 {
			return nil, codec.ErrTooManyElements
		}
		Entries = optional.Some(_list)
	}
	_lEntries, err := codec.MakeOptionalList[CodecTlvChainPresentIfBasicEntriesBound](codec.OptionalSourcesOf(Entries, func(e *codec_tlv_entry.CodecTlvEntry) codec_tlv_entry.CodecTlvEntrySource { return e }))
	if err != nil {
		return nil, err
	}
	return &CodecTlvChainPresentIfBasic{
		carrier: Carrier,
		entries: _lEntries,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecTlvChainPresentIfBasic) HasChain() bool {
	return (s.carrier & 0x01) != 0
}

func (s *CodecTlvChainPresentIfBasic) SetHasChain(v bool) {
	if v {
		s.carrier |= 0x01
	} else {
		s.carrier &^= 0x01
	}
}

// Encode writes the CodecTlvChainPresentIfBasic into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecTlvChainPresentIfBasic) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecTlvChainPresentIfBasic) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 137)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
