// SCE-MAP: codec_present_if_disjunction:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_present_if_disjunction

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
)

// CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsA is the FlagsWantsAOrWantsB group of CodecPresentIfDisjunction where `flags.wants_a` holds and `flags.wants_b` does not hold (docs/adr/0065 §6 R4): the
// members present then.
type CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsA struct {
	seq uint16
}

// MakeCodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsA returns the arm that holds each member it is handed.
func MakeCodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsA(seq uint16) CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsA {
	return CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsA{seq: seq}
}

// Seq returns the member the arm presents as Seq.
func (a CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsA) Seq() uint16 {
	return a.seq
}

func (CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsA) isCodecPresentIfDisjunctionFlagsWantsAOrWantsB() {}

// CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsB is the FlagsWantsAOrWantsB group of CodecPresentIfDisjunction where `flags.wants_a` does not hold and `flags.wants_b` holds (docs/adr/0065 §6 R4): the
// members present then.
type CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsB struct {
	seq uint16
}

// MakeCodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsB returns the arm that holds each member it is handed.
func MakeCodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsB(seq uint16) CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsB {
	return CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsB{seq: seq}
}

// Seq returns the member the arm presents as Seq.
func (a CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsB) Seq() uint16 {
	return a.seq
}

func (CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsB) isCodecPresentIfDisjunctionFlagsWantsAOrWantsB() {}

// CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsAWantsB is the FlagsWantsAOrWantsB group of CodecPresentIfDisjunction where `flags.wants_a` holds and `flags.wants_b` holds (docs/adr/0065 §6 R4): the
// members present then.
type CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsAWantsB struct {
	seq uint16
}

// MakeCodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsAWantsB returns the arm that holds each member it is handed.
func MakeCodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsAWantsB(seq uint16) CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsAWantsB {
	return CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsAWantsB{seq: seq}
}

// Seq returns the member the arm presents as Seq.
func (a CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsAWantsB) Seq() uint16 {
	return a.seq
}

func (CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsAWantsB) isCodecPresentIfDisjunctionFlagsWantsAOrWantsB() {}

// CodecPresentIfDisjunctionFlagsWantsAOrWantsB is the FlagsWantsAOrWantsB group of CodecPresentIfDisjunction: the arm the value holds, each the
// members present while the gates hold its values (docs/adr/0065 §6 R4),
// one of CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsA, CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsB, CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsAWantsB, which only this package declares.
type CodecPresentIfDisjunctionFlagsWantsAOrWantsB interface {
	isCodecPresentIfDisjunctionFlagsWantsAOrWantsB()
}

// CodecPresentIfDisjunctionSource is what the encode of CodecPresentIfDisjunction reads: one method
// for each element a value presents (docs/adr/0081 §2). CodecPresentIfDisjunction is one,
// and so is any value with these methods. The encode derives every other
// element of the frame.
type CodecPresentIfDisjunctionSource interface {
	Flags() uint8
	FlagsWantsAOrWantsB() optional.Of[CodecPresentIfDisjunctionFlagsWantsAOrWantsB]
}

// CodecPresentIfDisjunction represents the codec frame layout.
type CodecPresentIfDisjunction struct {
	flags uint8
	flagsWantsAOrWantsB optional.Of[CodecPresentIfDisjunctionFlagsWantsAOrWantsB]
}

// MakeCodecPresentIfDisjunction returns the CodecPresentIfDisjunction that presents each element it
// is handed, in declaration order (docs/adr/0081 §2). Its fields are
// unexported: outside this package a value is made here, by
// DecodeCodecPresentIfDisjunction, or as its zero value, and is read through its methods.
func MakeCodecPresentIfDisjunction(flags uint8, flagsWantsAOrWantsB optional.Of[CodecPresentIfDisjunctionFlagsWantsAOrWantsB]) CodecPresentIfDisjunction {
	return CodecPresentIfDisjunction{flags: flags, flagsWantsAOrWantsB: flagsWantsAOrWantsB}
}

// Flags returns the element the value presents as Flags.
func (s *CodecPresentIfDisjunction) Flags() uint8 {
	return s.flags
}

// FlagsWantsAOrWantsB returns the element the value presents as FlagsWantsAOrWantsB.
func (s *CodecPresentIfDisjunction) FlagsWantsAOrWantsB() optional.Of[CodecPresentIfDisjunctionFlagsWantsAOrWantsB] {
	return s.flagsWantsAOrWantsB
}

// EncodeSource writes any source of CodecPresentIfDisjunction into w, each element read
// off src and written where the codec's encode plan lays it out
// (docs/adr/0081). It refuses no value: it returns only the
// error of w, codec.ErrBufferOverflow from a bounded sink
// that has no room left.
func EncodeSource(src CodecPresentIfDisjunctionSource, w codec.SceSink) error {
	_rFlagsWantsAOrWantsB := src.FlagsWantsAOrWantsB()
	var _mSeq optional.Of[uint16]
	if _h, ok := _rFlagsWantsAOrWantsB.Get(); ok {
		switch _g := _h.(type) {
		case CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsA:
			_mSeq = optional.Some(_g.Seq())
		case CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsB:
			_mSeq = optional.Some(_g.Seq())
		case CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsAWantsB:
			_mSeq = optional.Some(_g.Seq())
		}
	}
	var _aFlagsWantsA uint64
	if _h, ok := _rFlagsWantsAOrWantsB.Get(); ok {
		switch _h.(type) {
		case CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsA, CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsAWantsB:
			_aFlagsWantsA = 1
		}
	}
	var _aFlagsWantsB uint64
	if _h, ok := _rFlagsWantsAOrWantsB.Get(); ok {
		switch _h.(type) {
		case CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsB, CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsAWantsB:
			_aFlagsWantsB = 1
		}
	}
	_cFlags := src.Flags()
	_cFlags = (_cFlags &^ 0x1) | (uint8(_aFlagsWantsA & 0x1) << 0)
	_cFlags = (_cFlags &^ 0x2) | (uint8(_aFlagsWantsB & 0x1) << 1)
	if err := w.WriteBytes([]byte{byte(_cFlags)}); err != nil {
		return err
	}
	if _mSeq.IsPresent() {
		if err := w.WriteBytes([]byte{byte(_mSeq.Proven() >> 8 & 0xFF), byte(_mSeq.Proven() & 0xFF)}); err != nil {
			return err
		}
	}
	return nil
}

// DecodeCodecPresentIfDisjunction decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §synth-5-B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecPresentIfDisjunction(cursor *codec.SceCursor) (*CodecPresentIfDisjunction, error) {
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
	var Flags uint8
	{
		raw, err := cursor.PeekSlice(1)
		if err != nil {
			return nil, err
		}
		Flags = raw[0]
		if err := cursor.Advance(1); err != nil {
			return nil, err
		}
	}
	var Seq optional.Of[uint16]
	if (Flags & 0x01) != 0 || (Flags & 0x02) != 0 {
		raw, err := cursor.PeekSlice(2)
		if err != nil {
			return nil, err
		}
		_v := uint16(raw[0])<<8 | uint16(raw[1])
		if err := cursor.Advance(2); err != nil {
			return nil, err
		}
		Seq = optional.Some(_v)
	}
	var _gFlagsWantsAOrWantsB optional.Of[CodecPresentIfDisjunctionFlagsWantsAOrWantsB]
	if ((Flags >> 0) & 1) != 0 && ((Flags >> 1) & 1) == 0 {
		_gFlagsWantsAOrWantsB = optional.Some[CodecPresentIfDisjunctionFlagsWantsAOrWantsB](MakeCodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsA(Seq.Proven()))
	} else if ((Flags >> 0) & 1) == 0 && ((Flags >> 1) & 1) != 0 {
		_gFlagsWantsAOrWantsB = optional.Some[CodecPresentIfDisjunctionFlagsWantsAOrWantsB](MakeCodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsB(Seq.Proven()))
	} else if ((Flags >> 0) & 1) != 0 && ((Flags >> 1) & 1) != 0 {
		_gFlagsWantsAOrWantsB = optional.Some[CodecPresentIfDisjunctionFlagsWantsAOrWantsB](MakeCodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsAWantsB(Seq.Proven()))
	}
	return &CodecPresentIfDisjunction{
		flags: Flags,
		flagsWantsAOrWantsB: _gFlagsWantsAOrWantsB,
	}, nil
}

// RFC §synth-5-B flags primitive: per-bit-range accessors over
// the carrier field. Single-bit (width=1) reads as bool; multi-bit
// (width>=2) reads as the smallest unsigned int type that fits. Setters
// mask + shift on the way in so out-of-range callers can't corrupt
// sibling bits. Wire layout is unchanged — the carrier still occupies
// its declared bytes.
func (s *CodecPresentIfDisjunction) WantsA() bool {
	return (s.flags & 0x01) != 0
}

func (s *CodecPresentIfDisjunction) SetWantsA(v bool) {
	if v {
		s.flags |= 0x01
	} else {
		s.flags &^= 0x01
	}
}

func (s *CodecPresentIfDisjunction) WantsB() bool {
	return (s.flags & 0x02) != 0
}

func (s *CodecPresentIfDisjunction) SetWantsB(v bool) {
	if v {
		s.flags |= 0x02
	} else {
		s.flags &^= 0x02
	}
}

// Encode writes the CodecPresentIfDisjunction into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecPresentIfDisjunction) Encode(w codec.SceSink) error {
	return EncodeSource(s, w)
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecPresentIfDisjunction) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 3)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
