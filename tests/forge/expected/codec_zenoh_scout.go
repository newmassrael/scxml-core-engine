// SCE-MAP: codec_zenoh_scout:41

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_scout

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
)

// CodecZenohScout represents the codec frame layout.
type CodecZenohScout struct {
	Version uint8
	Cbyte uint8
	Zid []byte
}

// DecodeCodecZenohScout decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohScout(cursor *codec.SceCursor) (*CodecZenohScout, error) {
	// RFC §synth-5-B present-if primitive: streaming decode
	// advances the cursor per field. Gated fields use `*T` for fixed
	// (nil = absent) or `[]byte` (nil = absent) for tail/length-ref;
	// VLE gating uses `*T` like fixed. Per-field `is_repeat` routes
	// Repeat fields to the dedicated helper. Branch fires before
	// has_vle_fields so a codec mixing VLE + present-if uses the
	// unified streaming path.
	var Version uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Version = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var Cbyte uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Cbyte = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var Zid []byte
	if (Cbyte & 0x08) != 0 {
		_n := (int((Cbyte >> 4) & 0xF) + 1)
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Zid = append([]byte(nil), raw...)
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	return &CodecZenohScout{
		Version: Version,
		Cbyte: Cbyte,
		Zid: Zid,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohScout) What() uint8 {
	return uint8((s.Cbyte >> 0) & 0x07)
}

func (s *CodecZenohScout) SetWhat(v uint8) {
	const _shiftedMask uint8 = 0x07 << 0
	_val := (uint8(v) & 0x07) << 0
	s.Cbyte = (s.Cbyte &^ _shiftedMask) | _val
}

func (s *CodecZenohScout) I() bool {
	return (s.Cbyte & 0x08) != 0
}

func (s *CodecZenohScout) SetI(v bool) {
	if v {
		s.Cbyte |= 0x08
	} else {
		s.Cbyte &^= 0x08
	}
}

func (s *CodecZenohScout) ZidLenM1() uint8 {
	return uint8((s.Cbyte >> 4) & 0x0F)
}

func (s *CodecZenohScout) SetZidLenM1(v uint8) {
	const _shiftedMask uint8 = 0x0F << 4
	_val := (uint8(v) & 0x0F) << 4
	s.Cbyte = (s.Cbyte &^ _shiftedMask) | _val
}

// Encode writes the CodecZenohScout into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohScout) Encode(w codec.SceSink) error {
	// RFC §synth-5-B present-if encode.
	if err := w.WriteBytes([]byte{ s.Version }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ s.Cbyte }); err != nil {
		return err
	}
	if s.Zid != nil {
		if err := w.WriteBytes(s.Zid); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohScout) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 18)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
