// SCE-MAP: codec_zenoh_scout:41 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_scout

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
)

// CodecZenohScoutZidBound is the bound of the zid element of CodecZenohScout: 1 to 16 bytes
// (docs/adr/0077 §3). A Bytes of it is made only within it.
type CodecZenohScoutZidBound struct{}

// Least is the fewest bytes the element holds.
func (CodecZenohScoutZidBound) Least() int { return 1 }

// Most is the most bytes the element holds.
func (CodecZenohScoutZidBound) Most() int { return 16 }

// CodecZenohScoutSource is what the encode of CodecZenohScout reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecZenohScout is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecZenohScoutSource interface {
	Version() uint8
	Cbyte() uint8
	Zid() optional.Of[codec.Bytes[CodecZenohScoutZidBound]]
}

// CodecZenohScout represents the codec frame layout.
type CodecZenohScout struct {
	version uint8
	cbyte uint8
	zid optional.Of[codec.Bytes[CodecZenohScoutZidBound]]
}

// MakeCodecZenohScout returns the CodecZenohScout that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecZenohScout, or as its zero value, and is read through its methods.
func MakeCodecZenohScout(version uint8, cbyte uint8, zid optional.Of[codec.Bytes[CodecZenohScoutZidBound]]) CodecZenohScout {
	return CodecZenohScout{version: version, cbyte: cbyte, zid: zid}
}

// Version returns the element the value presents as Version.
func (s *CodecZenohScout) Version() uint8 {
	return s.version
}

// Cbyte returns the element the value presents as Cbyte.
func (s *CodecZenohScout) Cbyte() uint8 {
	return s.cbyte
}

// Zid returns the element the value presents as Zid.
func (s *CodecZenohScout) Zid() optional.Of[codec.Bytes[CodecZenohScoutZidBound]] {
	return s.zid
}

// EncodeSource writes any source of CodecZenohScout into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecZenohScoutSource, w codec.SceSink) error {
	_cCbyte := src.Cbyte()
	_cCbyte &^= 0x8
	if src.Zid().IsPresent() {
		_cCbyte |= 0x8
	}
	if src.Zid().IsPresent() {
		_cCbyte = (_cCbyte &^ 0xF0) | (uint8(src.Zid().Proven().Len() - 1) << 4)
	}
	if err := w.WriteBytes([]byte{byte(src.Version())}); err != nil {
		return err
	}
	if err := w.WriteBytes([]byte{byte(_cCbyte)}); err != nil {
		return err
	}
	if src.Zid().IsPresent() {
		if err := w.WriteBytes(src.Zid().Proven().Bytes()); err != nil {
			return err
		}
	}
	return nil
}

// DecodeCodecZenohScout decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohScout(cursor *codec.SceCursor) (*CodecZenohScout, error) {
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
	var Zid optional.Of[[]byte]
	if (Cbyte & 0x08) != 0 {
		_len := int((Cbyte >> 4) & 0xF)
		_n := _len + 1
		raw, err := cursor.PeekSlice(_n)
		if err != nil {
			return nil, err
		}
		Zid = optional.Some(append([]byte(nil), raw...))
		if err := cursor.Advance(_n); err != nil {
			return nil, err
		}
	}
	_bZid, err := codec.MakeOptionalBytes[CodecZenohScoutZidBound](Zid)
	if err != nil {
		return nil, err
	}
	return &CodecZenohScout{
		version: Version,
		cbyte: Cbyte,
		zid: _bZid,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecZenohScout) What() uint8 {
	return uint8((s.cbyte >> 0) & 0x07)
}

func (s *CodecZenohScout) SetWhat(v uint8) {
	const _shiftedMask uint8 = 0x07 << 0
	_val := (uint8(v) & 0x07) << 0
	s.cbyte = (s.cbyte &^ _shiftedMask) | _val
}

func (s *CodecZenohScout) I() bool {
	return (s.cbyte & 0x08) != 0
}

func (s *CodecZenohScout) SetI(v bool) {
	if v {
		s.cbyte |= 0x08
	} else {
		s.cbyte &^= 0x08
	}
}

func (s *CodecZenohScout) ZidLenM1() uint8 {
	return uint8((s.cbyte >> 4) & 0x0F)
}

func (s *CodecZenohScout) SetZidLenM1(v uint8) {
	const _shiftedMask uint8 = 0x0F << 4
	_val := (uint8(v) & 0x0F) << 4
	s.cbyte = (s.cbyte &^ _shiftedMask) | _val
}

// Encode writes the CodecZenohScout into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohScout) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
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
