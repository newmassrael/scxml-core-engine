// SCE-MAP: codec_chain_has_tagged_entry:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_chain_has_tagged_entry

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
)

// CodecChainHasTaggedEntrySource is what the encode of CodecChainHasTaggedEntry reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecChainHasTaggedEntry is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecChainHasTaggedEntrySource interface {
	EntryType() uint8
	Ctl() uint8
	Body() uint8
}

// CodecChainHasTaggedEntry represents the codec frame layout.
type CodecChainHasTaggedEntry struct {
	entryType uint8
	ctl uint8
	body uint8
}

// MakeCodecChainHasTaggedEntry returns the CodecChainHasTaggedEntry that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecChainHasTaggedEntry, or as its zero value, and is read through its methods.
func MakeCodecChainHasTaggedEntry(entryType uint8, ctl uint8, body uint8) CodecChainHasTaggedEntry {
	return CodecChainHasTaggedEntry{entryType: entryType, ctl: ctl, body: body}
}

// EntryType returns the element the value presents as EntryType.
func (s *CodecChainHasTaggedEntry) EntryType() uint8 {
	return s.entryType
}

// Ctl returns the element the value presents as Ctl.
func (s *CodecChainHasTaggedEntry) Ctl() uint8 {
	return s.ctl
}

// Body returns the element the value presents as Body.
func (s *CodecChainHasTaggedEntry) Body() uint8 {
	return s.body
}

// CodecChainHasTaggedEntryOverrides holds the one-bit flags of CodecChainHasTaggedEntry a parent may
// set in place of the ones a value presents, such as the flag a chain whose
// entries are CodecChainHasTaggedEntry ends them on, which the chain derives from where
// each entry stands. A flag it leaves absent is the one the value presents.
type CodecChainHasTaggedEntryOverrides struct {
	// CtlMore is the `more` flag of `ctl`, where a parent sets it.
	CtlMore optional.Of[bool]
}

// EncodeSource writes any source of CodecChainHasTaggedEntry into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecChainHasTaggedEntrySource, w codec.SceSink) error {
	return EncodeSourceOverriding(src, w, CodecChainHasTaggedEntryOverrides{})
}

// EncodeSourceOverriding is EncodeSource, each flag overrides sets written in
// place of the one src presents.
func EncodeSourceOverriding(src CodecChainHasTaggedEntrySource, w codec.SceSink, overrides CodecChainHasTaggedEntryOverrides) error {
	_cCtl := src.Ctl()
	if _o, ok := overrides.CtlMore.Get(); ok {
		_cCtl &^= 0x1
		if _o {
			_cCtl |= 0x1
		}
	}
	if err := w.WriteBytes([]byte{byte(src.EntryType())}); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{byte(_cCtl)}); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{byte(src.Body())}); err != nil {
		return err
	}
	return nil
}

// DecodeCodecChainHasTaggedEntry decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecChainHasTaggedEntry(cursor *codec.SceCursor) (*CodecChainHasTaggedEntry, error) {
	raw, err := cursor.PeekSlice(3)
	if err != nil {
		return nil, err
	}
	EntryType := raw[0]
	Ctl := raw[1]
	Body := raw[2]
	value := &CodecChainHasTaggedEntry{
		entryType: EntryType,
		ctl: Ctl,
		body: Body,
	}
	if err := cursor.Advance(3); err != nil {
		return nil, err
	}
	return value, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecChainHasTaggedEntry) More() bool {
	return (s.ctl & 0x01) != 0
}

func (s *CodecChainHasTaggedEntry) SetMore(v bool) {
	if v {
		s.ctl |= 0x01
	} else {
		s.ctl &^= 0x01
	}
}

// Encode writes the CodecChainHasTaggedEntry into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecChainHasTaggedEntry) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecChainHasTaggedEntry) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 3)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
