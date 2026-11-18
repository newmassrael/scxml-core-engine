// SCE-MAP: codec_tlv_entry:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_tlv_entry

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecTlvEntryEntryBodyBound is the bound of the entry_body element of CodecTlvEntry: 0 to 32 bytes
// (docs/adr/0077 §3). A Bytes of it is made only within it.
type CodecTlvEntryEntryBodyBound struct{}

// Least is the fewest bytes the element holds.
func (CodecTlvEntryEntryBodyBound) Least() int { return 0 }

// Most is the most bytes the element holds.
func (CodecTlvEntryEntryBodyBound) Most() int { return 32 }

// CodecTlvEntrySource is what the encode of CodecTlvEntry reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecTlvEntry is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecTlvEntrySource interface {
	EntryType() uint8
	EntryBody() codec.Bytes[CodecTlvEntryEntryBodyBound]
}

// CodecTlvEntry represents the codec frame layout.
type CodecTlvEntry struct {
	entryType uint8
	entryBody codec.Bytes[CodecTlvEntryEntryBodyBound]
}

// MakeCodecTlvEntry returns the CodecTlvEntry that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecTlvEntry, or as its zero value, and is read through its methods.
func MakeCodecTlvEntry(entryType uint8, entryBody codec.Bytes[CodecTlvEntryEntryBodyBound]) CodecTlvEntry {
	return CodecTlvEntry{entryType: entryType, entryBody: entryBody}
}

// EntryType returns the element the value presents as EntryType.
func (s *CodecTlvEntry) EntryType() uint8 {
	return s.entryType
}

// EntryBody returns the element the value presents as EntryBody.
func (s *CodecTlvEntry) EntryBody() codec.Bytes[CodecTlvEntryEntryBodyBound] {
	return s.entryBody
}

// WireEntryLen returns the entry_len the frame of the value carries, which the
// encode derives from what it measures (docs/adr/0081 §3).
func (s *CodecTlvEntry) WireEntryLen() uint8 {
	return uint8(s.EntryBody().Len())
}

// EncodeSource writes any source of CodecTlvEntry into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It derives entry_len from
// what each measures, and never reads it off the source. It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecTlvEntrySource, w codec.SceSink) error {
	_dEntryLen := uint8(src.EntryBody().Len())
	if err := w.WriteBytes([]byte{byte(src.EntryType())}); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{byte(_dEntryLen)}); err != nil {
		return err
	}
	if err := w.WriteBytes(src.EntryBody().Bytes()); err != nil {
		return err
	}
	return nil
}

// DecodeCodecTlvEntry decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecTlvEntry(cursor *codec.SceCursor) (*CodecTlvEntry, error) {
	frameLen := cursor.Remaining()
	if frameLen < 2 {
		return nil, codec.ErrNeedMoreBytes
	}
	raw, err := cursor.PeekSlice(frameLen)
	if err != nil {
		return nil, err
	}
	EntryType := raw[0]
	EntryLen := raw[1]
	if EntryLen > 32 {
		return nil, codec.ErrLengthOutOfRange
	}
	if frameLen < 2+int(EntryLen) {
		return nil, codec.ErrNeedMoreBytes
	}
	EntryBody := raw[2:2+int(EntryLen)]
	_bEntryBody, err := codec.MakeBytes[CodecTlvEntryEntryBodyBound](EntryBody)
	if err != nil {
		return nil, err
	}
	value := &CodecTlvEntry{
		entryType: EntryType,
		entryBody: _bEntryBody,
	}
	// The bytes it read and no more (RFC §synth-5-B B3, as the Rust decode
	// does): the end of the last length-ref field, at least the fixed fields'
	// bytes, so that a codec holding this one as a body, an entry or an arm
	// decodes on after it. Each end is an argument of `max`, where gofmt
	// spells it as the guard above does.
	consumed := 2
	consumed = max(consumed, 2+int(EntryLen))
	if err := cursor.Advance(consumed); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode writes the CodecTlvEntry into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecTlvEntry) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecTlvEntry) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 34)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
