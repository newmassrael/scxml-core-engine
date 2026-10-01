// SCE-MAP: codec_chain_has_tagged_entry:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_chain_has_tagged_entry

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecChainHasTaggedEntry represents the codec frame layout.
type CodecChainHasTaggedEntry struct {
	EntryType uint8
	Ctl uint8
	Body uint8
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
		EntryType: EntryType,
		Ctl: Ctl,
		Body: Body,
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
	return (s.Ctl & 0x01) != 0
}

func (s *CodecChainHasTaggedEntry) SetMore(v bool) {
	if v {
		s.Ctl |= 0x01
	} else {
		s.Ctl &^= 0x01
	}
}

// Encode writes the CodecChainHasTaggedEntry into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecChainHasTaggedEntry) Encode(w codec.SceSink) error {
	if err := w.WriteBytes([]byte{ byte(s.EntryType) }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ byte(s.Ctl) }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ byte(s.Body) }); err != nil {
		return err
	}
	return nil
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
