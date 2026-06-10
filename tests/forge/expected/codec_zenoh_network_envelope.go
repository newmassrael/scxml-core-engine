// SCE-MAP: codec_zenoh_network_envelope:60

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package codec_zenoh_network_envelope

import (
	"github.com/newmassrael/sce-forge-runtime/codec"
	"example.com/sce-forge/codec_zenoh_interest"
	"example.com/sce-forge/codec_zenoh_response_final"
	"example.com/sce-forge/codec_zenoh_response"
	"example.com/sce-forge/codec_zenoh_request"
	"example.com/sce-forge/codec_zenoh_push"
	"example.com/sce-forge/codec_zenoh_declare"
	"example.com/sce-forge/codec_zenoh_oam"
)

// CodecZenohNetworkEnvelopeDefault bundles the runtime
// tag value with the catch-all body so encode can round-trip the
// observed tag back onto the wire (RFC §5.B variant primitive).
type CodecZenohNetworkEnvelopeDefault struct {
	Tag uint8
	Body codec_zenoh_oam.CodecZenohOam
}

// CodecZenohNetworkEnvelopeVariant is a discriminated-union body for the codec's
// tag-field suffix (RFC §5.B variant primitive). Exactly one of
// the pointer fields is non-nil at a time; the active arm is the one
// that matches the current tag value.
type CodecZenohNetworkEnvelopeVariant struct {
	CodecZenohInterest *codec_zenoh_interest.CodecZenohInterest
	CodecZenohResponseFinal *codec_zenoh_response_final.CodecZenohResponseFinal
	CodecZenohResponse *codec_zenoh_response.CodecZenohResponse
	CodecZenohRequest *codec_zenoh_request.CodecZenohRequest
	CodecZenohPush *codec_zenoh_push.CodecZenohPush
	CodecZenohDeclare *codec_zenoh_declare.CodecZenohDeclare
	CodecZenohOam *codec_zenoh_oam.CodecZenohOam
	Default *CodecZenohNetworkEnvelopeDefault
}

// CodecZenohNetworkEnvelope represents the codec frame layout.
type CodecZenohNetworkEnvelope struct {
	Body CodecZenohNetworkEnvelopeVariant
}

// NewCodecZenohNetworkEnvelope returns a CodecZenohNetworkEnvelope initialized with the
// declared wire-MID defaults. Go has no Default trait — round-trip
// safety (`NewCodecZenohNetworkEnvelope().Encode()` decodes back to the same
// arm) requires using this constructor rather than the bare struct
// literal `CodecZenohNetworkEnvelope{}`, which would zero-init every field
// (and leave every Variant arm pointer nil for variant codecs).
// RFC variant-default-uniformity (Go).
func NewCodecZenohNetworkEnvelope() *CodecZenohNetworkEnvelope {
	return &CodecZenohNetworkEnvelope{
		Body: CodecZenohNetworkEnvelopeVariant{
			CodecZenohOam: codec_zenoh_oam.NewCodecZenohOam(),
		},
	}
}

// DecodeCodecZenohNetworkEnvelope decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohNetworkEnvelope(cursor *codec.SceCursor) (*CodecZenohNetworkEnvelope, error) {
	// RFC §5.B peek-byte / streaming-prefix:
	// streaming prefix decode (variable-length fields supported via
	// per-field present_if/tlv-chain/embed/repeat helpers). Peek-byte
	// mode additionally peeks the cursor's next byte for variant tag
	// without advancing — arm body decoder reads it as own header.
	_peekSlice, err := cursor.PeekSlice(1)
	if err != nil {
		return nil, err
	}
	_peek := _peekSlice[0]
	// Dispatch on the tag field; each arm decodes its body codec from
	// the cursor. The default arm (when declared) carries the runtime
	// tag value so encode can round-trip it back onto the wire.
	body := CodecZenohNetworkEnvelopeVariant{}
	switch uint8((_peek >> 0) & 0x1F) {
	case 25:
		_arm, err := codec_zenoh_interest.DecodeCodecZenohInterest(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohInterest = _arm
	case 26:
		_arm, err := codec_zenoh_response_final.DecodeCodecZenohResponseFinal(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohResponseFinal = _arm
	case 27:
		_arm, err := codec_zenoh_response.DecodeCodecZenohResponse(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohResponse = _arm
	case 28:
		_arm, err := codec_zenoh_request.DecodeCodecZenohRequest(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohRequest = _arm
	case 29:
		_arm, err := codec_zenoh_push.DecodeCodecZenohPush(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohPush = _arm
	case 30:
		_arm, err := codec_zenoh_declare.DecodeCodecZenohDeclare(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohDeclare = _arm
	case 31:
		_arm, err := codec_zenoh_oam.DecodeCodecZenohOam(cursor)
		if err != nil {
			return nil, err
		}
		body.CodecZenohOam = _arm
	default:
		_arm, err := codec_zenoh_oam.DecodeCodecZenohOam(cursor)
		if err != nil {
			return nil, err
		}
		body.Default = &CodecZenohNetworkEnvelopeDefault{
			Tag: uint8((_peek >> 0) & 0x1F),
			Body: *_arm,
		}
	}
	return &CodecZenohNetworkEnvelope{
		Body: body,
	}, nil
}

// Encode writes the CodecZenohNetworkEnvelope into the caller-owned sink.
// Returns nil on success; codec.ErrBufferOverflow from a bounded sink
// when the destination has insufficient remaining capacity; growable
// sinks (e.g. BytesSink) are effectively infallible.
func (s *CodecZenohNetworkEnvelope) Encode(w codec.SceSink) error {
	// RFC §5.B peek-byte / streaming-prefix.
	// Append the active arm body's encoded bytes via the same sink.
	switch {
	case s.Body.CodecZenohInterest != nil:
		if err := s.Body.CodecZenohInterest.Encode(w); err != nil {
			return err
		}
	case s.Body.CodecZenohResponseFinal != nil:
		if err := s.Body.CodecZenohResponseFinal.Encode(w); err != nil {
			return err
		}
	case s.Body.CodecZenohResponse != nil:
		if err := s.Body.CodecZenohResponse.Encode(w); err != nil {
			return err
		}
	case s.Body.CodecZenohRequest != nil:
		if err := s.Body.CodecZenohRequest.Encode(w); err != nil {
			return err
		}
	case s.Body.CodecZenohPush != nil:
		if err := s.Body.CodecZenohPush.Encode(w); err != nil {
			return err
		}
	case s.Body.CodecZenohDeclare != nil:
		if err := s.Body.CodecZenohDeclare.Encode(w); err != nil {
			return err
		}
	case s.Body.CodecZenohOam != nil:
		if err := s.Body.CodecZenohOam.Encode(w); err != nil {
			return err
		}
	case s.Body.Default != nil:
		if err := s.Body.Default.Body.Encode(w); err != nil {
			return err
		}
	}
	return nil
}

// EncodeToBytes is the heap-backed convenience facade. Runs Encode
// over a BytesSink and returns the freshly-encoded byte slice.
// Callers targeting zero-alloc hot paths should call Encode directly
// against a caller-owned sink (e.g. BoundedSink over a stack buffer).
func (s *CodecZenohNetworkEnvelope) EncodeToBytes() []byte {
	_dst := make([]byte, 0, 1218)
	_ = s.Encode(codec.NewBytesSink(&_dst))
	return _dst
}
