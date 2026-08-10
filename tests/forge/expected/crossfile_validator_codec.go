// SCE-MAP: crossfile_validator_codec:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package crossfile_validator_codec

import (
	"example.com/sce-forge/codec_simple_frame"
)

// ValidationResult holds the outcome of a validation check.
type ValidationResult struct {
	Valid  bool
	Reason string
}

// CrossfileValidatorCodec performs range, rate-of-change, and plausibility validation.
type CrossfileValidatorCodec struct {
	// Imported kinds (cross-file composition)
	Frame codec_simple_frame.CodecSimpleFrame
}

// NewCrossfileValidatorCodec returns an initialized validator. Stateful imports
// whose Go zero-value is not a valid initial state (e.g. filter — its
// internal pointer to the runtime implementation is nil after zero-init
// and the first Update call would deref nil) opt in to an explicit
// factory call via `ImportContext::go_init_expr`. Codec imports leave
// their slot zero-initialized (the pure-data struct's natural empty
// state). Validators with no stateful imports get an empty literal,
// matching the legacy `var v X` zero-value path callers used before
// this constructor existed.
func NewCrossfileValidatorCodec() *CrossfileValidatorCodec {
	return &CrossfileValidatorCodec{
	}
}

// Validate checks all validation rules and returns the result.
func (p *CrossfileValidatorCodec) Validate(msgId uint8, payload uint16) ValidationResult {
	if msgId < 0 || msgId > 255 {
		return ValidationResult{Valid: false, Reason: "msg_id_out_of_range"}
	}
	if payload < 0 || payload > 4095 {
		return ValidationResult{Valid: false, Reason: "payload_out_of_range"}
	}
	if !(p.Frame.MsgId == msgId && p.Frame.Payload == payload) {
		return ValidationResult{Valid: false, Reason: "plausibility_failed"}
	}
	return ValidationResult{Valid: true, Reason: ""}
}
