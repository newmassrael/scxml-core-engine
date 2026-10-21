// SCE-GENERATED — DO NOT EDIT
// source-hash: 09d8fec62ddcc806d17c4e1da241b802dbe4f5317b3bd415a705b2ad99d82f3e
// SCE-MAP: forge_call_range.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package forge_call_range

// ValidationResult holds the outcome of a validation check.
type ValidationResult struct {
	Valid  bool
	Reason string
}

// ForgeCallRange performs range, rate-of-change, and plausibility validation.
type ForgeCallRange struct {
}

// NewForgeCallRange returns an initialized validator. Stateful imports
// whose Go zero-value is not a valid initial state (e.g. filter — its
// internal pointer to the runtime implementation is nil after zero-init
// and the first Update call would deref nil) opt in to an explicit
// factory call via `ImportContext::go_init_expr`. Codec imports leave
// their slot zero-initialized (the pure-data struct's natural empty
// state). Validators with no stateful imports get an empty literal,
// matching the legacy `var v X` zero-value path callers used before
// this constructor existed.
func NewForgeCallRange() *ForgeCallRange {
	return &ForgeCallRange{
	}
}

// Validate checks all validation rules and returns the result.
func (sceSelf *ForgeCallRange) Validate(level int32, offset int32) ValidationResult {
	if level > 30 {
		return ValidationResult{Valid: false, Reason: "level_out_of_range"}
	}
	if offset < 0 {
		return ValidationResult{Valid: false, Reason: "offset_out_of_range"}
	}
	return ValidationResult{Valid: true, Reason: ""}
}
