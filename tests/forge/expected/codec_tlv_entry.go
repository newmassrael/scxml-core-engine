// SCE-MAP: codec_tlv_entry:10

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_tlv_entry

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecTlvEntry represents the codec frame layout.
type CodecTlvEntry struct {
	EntryType uint8
	EntryLen uint8
	EntryBody []byte
}

// DecodeCodecTlvEntry decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
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
	EntryBody := raw[2:2+int(EntryLen)]
	value := &CodecTlvEntry{
		EntryType: EntryType,
		EntryLen: EntryLen,
		EntryBody: EntryBody,
	}
	if err := cursor.Advance(frameLen); err != nil {
		return nil, err
	}
	return value, nil
}

// Encode writes the CodecTlvEntry into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecTlvEntry) Encode(w codec.SceSink) error {
	if err := w.WriteBytes([]byte{ byte(s.EntryType) }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ byte(s.EntryLen) }); err != nil {
		return err
	}
	if err := w.WriteBytes(s.EntryBody); err != nil {
		return err
	}
	return nil
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
