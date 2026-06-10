// SCE-MAP: codec_zenoh_hello:41

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_hello

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_locator"
)

// CodecZenohHello represents the codec frame layout.
type CodecZenohHello struct {
	Version uint8
	Cbyte uint8
	Zid []byte
	NumLocators *uint64
	Locators []codec_zenoh_locator.CodecZenohLocator
}

// DecodeCodecZenohHello decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohHello(cursor *codec.SceCursor, L byte) (*CodecZenohHello, error) {
	// RFC §5.B present-if primitive: streaming decode
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
	{
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
	var NumLocators *uint64
	if (L & 0x01) != 0 {
		_v, err := cursor.ReadVLEU64()
	if err != nil { return nil, err }
		NumLocators = &_v
	}
	var Locators []codec_zenoh_locator.CodecZenohLocator
	if (L & 0x01) != 0 {
		_n := *NumLocators
		Locators = make([]codec_zenoh_locator.CodecZenohLocator, 0, _n)
		for _i := 0; _i < int(_n); _i++ {
			_elem, err := codec_zenoh_locator.DecodeCodecZenohLocator(cursor)
			if err != nil {
				return nil, err
			}
			Locators = append(Locators, *_elem)
		}
	}
	return &CodecZenohHello{
		Version: Version,
		Cbyte: Cbyte,
		Zid: Zid,
		NumLocators: NumLocators,
		Locators: Locators,
	}, nil
}

// RFC §5.B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohHello) Whatami() uint8 {
	return uint8((s.Cbyte >> 0) & 0x03)
}

func (s *CodecZenohHello) SetWhatami(v uint8) {
	const _shiftedMask uint8 = 0x03 << 0
	_val := (uint8(v) & 0x03) << 0
	s.Cbyte = (s.Cbyte &^ _shiftedMask) | _val
}

func (s *CodecZenohHello) ZidLenM1() uint8 {
	return uint8((s.Cbyte >> 4) & 0x0F)
}

func (s *CodecZenohHello) SetZidLenM1(v uint8) {
	const _shiftedMask uint8 = 0x0F << 4
	_val := (uint8(v) & 0x0F) << 4
	s.Cbyte = (s.Cbyte &^ _shiftedMask) | _val
}

// Encode writes the CodecZenohHello into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohHello) Encode(w codec.SceSink, L byte) error {
	// RFC §5.B present-if encode.
	if err := w.WriteBytes([]byte{ s.Version }); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{ s.Cbyte }); err != nil {
		return err
	}
	if err := w.WriteBytes(s.Zid); err != nil {
		return err
	}
	if s.NumLocators != nil {
		_v := *s.NumLocators
	{
		_vle := uint64(_v)
		for _vle >= 0x80 {
			if err := w.WriteBytes([]byte{ byte(_vle&0x7F) | 0x80 }); err != nil {
				return err
			}
			_vle >>= 7
		}
		if err := w.WriteBytes([]byte{ byte(_vle) }); err != nil {
			return err
		}
	}
	}
	if s.Locators != nil {
		for _i := range s.Locators {
			if err := s.Locators[_i].Encode(w); err != nil {
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
func (s *CodecZenohHello) EncodeToBytes(L byte) []byte {
	_dst := make([]byte, 0, 8860)
	_ = s.Encode(codec.NewBytesSink(&_dst), L)
	return _dst
}
