// SCE-MAP: codec_until_eof_basic:10

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_until_eof_basic

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_repeat_elem"
)

// CodecUntilEofBasic represents the codec frame layout.
type CodecUntilEofBasic struct {
	Msgs []codec_repeat_elem.CodecRepeatElem
}

// DecodeCodecUntilEofBasic decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecUntilEofBasic(cursor *codec.SceCursor) (*CodecUntilEofBasic, error) {
	// RFC §5.B B2 repeat primitive: streaming decode mixes plain
	// fixed-width reads (per-field via the present-if helper's
	// non-gated arm) with `make([]T, 0, N)` + `append` loops that
	// iterate the imported codec's `Decode<T>(cursor)` either
	// `int(N)` times (length-field) or until `cursor.Remaining() == 0`
	// (until-eof). Element bodies recurse into their own decoder —
	// each may itself surface `codec.ErrNeedMoreBytes`, unwinding
	// the partial frame.
	Msgs := make([]codec_repeat_elem.CodecRepeatElem, 0)
	for cursor.Remaining() > 0 {
		_elem, err := codec_repeat_elem.DecodeCodecRepeatElem(cursor)
		if err != nil {
			return nil, err
		}
		Msgs = append(Msgs, *_elem)
	}
	return &CodecUntilEofBasic{
		Msgs: Msgs,
	}, nil
}

// Encode serializes the CodecUntilEofBasic into raw bytes.
func (s *CodecUntilEofBasic) Encode() []byte {
	// RFC §5.B B2 encode: fixed prefix appends byte-by-byte; repeat
	// fields range over `s.<Pascal>` and spread each element's
	// `Encode()` bytes into the parent buffer. Author keeps count
	// field == slice length (trust contract).
	r := make([]byte, 0, 128)
	for _, _e := range s.Msgs {
		r = append(r, _e.Encode()...)
	}
	return r
}
