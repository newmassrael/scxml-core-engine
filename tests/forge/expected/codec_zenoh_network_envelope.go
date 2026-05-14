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
// observed tag back onto the wire (RFC §5.B variant primitive B1-β).
type CodecZenohNetworkEnvelopeDefault struct {
	Tag uint8
	Body codec_zenoh_oam.CodecZenohOam
}

// CodecZenohNetworkEnvelopeVariant is a discriminated-union body for the codec's
// tag-field suffix (RFC §5.B variant primitive B1-β). Exactly one of
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

// DecodeCodecZenohNetworkEnvelope decodes the next frame from cursor.
// On success the cursor advances past the consumed bytes; returns
// `codec.ErrNeedMoreBytes` (without advancing) when the cursor's tail
// is shorter than the declared minimum frame (RFC §5.B L494-519).
// VLE codecs may also return `codec.ErrVLEWidthOverflow`.
func DecodeCodecZenohNetworkEnvelope(cursor *codec.SceCursor) (*CodecZenohNetworkEnvelope, error) {
	// RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
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

// Encode serializes the CodecZenohNetworkEnvelope into raw bytes.
func (s *CodecZenohNetworkEnvelope) Encode() []byte {
	// RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
	// streaming prefix encode. Peek-byte mode: arm body's encode
	// prepends its own header byte (which the decoder peeked); no
	// separate tag byte here. Streaming-prefix mode (own-field):
	// carrier is part of the prefix fields and emits via the same
	// per-field path.
	r := make([]byte, 0, 1218)
	// Append the active arm body's encoded bytes.
	switch {
	case s.Body.CodecZenohInterest != nil:
		r = append(r, s.Body.CodecZenohInterest.Encode()...)
	case s.Body.CodecZenohResponseFinal != nil:
		r = append(r, s.Body.CodecZenohResponseFinal.Encode()...)
	case s.Body.CodecZenohResponse != nil:
		r = append(r, s.Body.CodecZenohResponse.Encode()...)
	case s.Body.CodecZenohRequest != nil:
		r = append(r, s.Body.CodecZenohRequest.Encode()...)
	case s.Body.CodecZenohPush != nil:
		r = append(r, s.Body.CodecZenohPush.Encode()...)
	case s.Body.CodecZenohDeclare != nil:
		r = append(r, s.Body.CodecZenohDeclare.Encode()...)
	case s.Body.CodecZenohOam != nil:
		r = append(r, s.Body.CodecZenohOam.Encode()...)
	case s.Body.Default != nil:
		r = append(r, s.Body.Default.Body.Encode()...)
	}
	return r
}
