// SCE-MAP: codec_chain_has_marker:28 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_chain_has_marker

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_ext_entry"
	"example.com/sce-forge/codec_chain_has_marker_slice"
)

// CodecChainHasMarker represents the codec frame layout.
type CodecChainHasMarker struct {
	Header uint8
	Extensions []codec_zenoh_ext_entry.CodecZenohExtEntry
	PayloadLen *uint64
	Payload []byte
	SliceCount *uint32
	Slices []codec_chain_has_marker_slice.CodecChainHasMarkerSlice
}

// DecodeCodecChainHasMarker decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecChainHasMarker(cursor *codec.SceCursor) (*CodecChainHasMarker, error) {
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
	var Header uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Header = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var Extensions []codec_zenoh_ext_entry.CodecZenohExtEntry
	if (Header & 0x80) != 0 {
		Extensions = make([]codec_zenoh_ext_entry.CodecZenohExtEntry, 0, 4)
		_more := false
		for _i := 0; _i < int(4); _i++ {
			if cursor.Remaining() == 0 {
				break
			}
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
		if _more {
			return nil, codec.ErrTlvChainOverflow
		}
	}
	_has_Extensions_18 := false
	for _, _e := range Extensions {
		if (uint64(_e.Header) & 127) == 18 {
			_has_Extensions_18 = true
			break
		}
	}
	var PayloadLen *uint64
	if !_has_Extensions_18 {
		_v, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
		PayloadLen = &_v
	}
	var Payload []byte
	if !_has_Extensions_18 {
		_n := int(*PayloadLen)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Payload = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	var SliceCount *uint32
	if _has_Extensions_18 {
		_v, err := cursor.ReadVLEU32()
	if err != nil { return nil, err }
		SliceCount = &_v
	}
	var Slices []codec_chain_has_marker_slice.CodecChainHasMarkerSlice
	if _has_Extensions_18 {
		_n := *SliceCount
		Slices = make([]codec_chain_has_marker_slice.CodecChainHasMarkerSlice, 0, _n)
		for _i := 0; _i < int(_n); _i++ {
			_elem, err := codec_chain_has_marker_slice.DecodeCodecChainHasMarkerSlice(cursor)
			if err != nil {
				return nil, err
			}
			Slices = append(Slices, *_elem)
		}
	}
	return &CodecChainHasMarker{
		Header: Header,
		Extensions: Extensions,
		PayloadLen: PayloadLen,
		Payload: Payload,
		SliceCount: SliceCount,
		Slices: Slices,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecChainHasMarker) Kind() uint8 {
	return uint8((s.Header >> 0) & 0x1F)
}

func (s *CodecChainHasMarker) SetKind(v uint8) {
	const _shiftedMask uint8 = 0x1F << 0
	_val := (uint8(v) & 0x1F) << 0
	s.Header = (s.Header &^ _shiftedMask) | _val
}

func (s *CodecChainHasMarker) E() bool {
	return (s.Header & 0x80) != 0
}

func (s *CodecChainHasMarker) SetE(v bool) {
	if v {
		s.Header |= 0x80
	} else {
		s.Header &^= 0x80
	}
}

// Encode writes the CodecChainHasMarker into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecChainHasMarker) Encode(w codec.SceSink) error {
	_has_Extensions_18 := false
	for _, _e := range s.Extensions {
		if (uint64(_e.Header) & 127) == 18 {
			_has_Extensions_18 = true
			break
		}
	}
	if !_has_Extensions_18 {
		if s.PayloadLen == nil {
			return codec.ErrPresentIfMismatch
		}
	} else if s.PayloadLen != nil {
		return codec.ErrPresentIfMismatch
	}
	if !_has_Extensions_18 {
		if s.Payload == nil {
			return codec.ErrPresentIfMismatch
		}
	} else if s.Payload != nil {
		return codec.ErrPresentIfMismatch
	}
	if _has_Extensions_18 {
		if s.SliceCount == nil {
			return codec.ErrPresentIfMismatch
		}
	} else if s.SliceCount != nil {
		return codec.ErrPresentIfMismatch
	}
	if _has_Extensions_18 {
		if s.Slices == nil {
			return codec.ErrPresentIfMismatch
		}
	} else if s.Slices != nil {
		return codec.ErrPresentIfMismatch
	}
	// Streaming cursor encode (SSOT selection: `needs_streaming`).
	// Mirrors the streaming decode: every field appends its own bytes in
	// declaration order through the per-field encode blocks, so a gated
	// field skips its append when absent, and a fixed field after a
	// variable-length payload lands after the payload (the positional path
	// appends variable fields last, placing it ahead on the wire).
	// Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
	// dedicated helpers; everything else uses `present_if_encode_block`.
	if err := w.WriteBytes([]byte{ s.Header }); err != nil {
		return err
	}
	for _i := range s.Extensions {
		if err := s.Extensions[_i].Encode(w); err != nil {
			return err
		}
	}
	if s.PayloadLen != nil {
		_v := *s.PayloadLen
	if err := codec.WriteVLEU64(w, uint64(_v)); err != nil {
		return err
	}
	}
	if s.Payload != nil {
		if err := w.WriteBytes(s.Payload); err != nil {
			return err
		}
	}
	if s.SliceCount != nil {
		_v := *s.SliceCount
	if err := codec.WriteVLEU32(w, uint32(_v)); err != nil {
		return err
	}
	}
	if s.Slices != nil {
		for _i := range s.Slices {
			if err := s.Slices[_i].Encode(w); err != nil {
				return err
			}
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice, or the
// error Encode refuses a message with when a field's presence disagrees
// with the chain-membership `sce:present-if` that gates it
// (codec.ErrPresentIfMismatch). Callers targeting zero-alloc hot paths
// should call Encode directly against a caller-owned sink (e.g.
// BoundedSink over a stack buffer).
func (s *CodecChainHasMarker) EncodeToBytes() ([]byte, error) {
	_dst := make([]byte, 0, 1511)
	if err := s.Encode(codec.NewBytesSink(&_dst)); err != nil {
		return nil, err
	}
	return _dst, nil
}
