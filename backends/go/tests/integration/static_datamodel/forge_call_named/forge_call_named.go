// SCE-GENERATED — DO NOT EDIT
// source-hash: 81a3de342b33c3189f4a5813894163bc1e16bfc77aa7ead74e709e8ba7b570b9
// SCE-MAP: forge_call_named.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package forge_call_named

// ValidationResult holds the outcome of a validation check.
type ValidationResult struct {
	Valid  bool
	Reason string
}

// ForgeCallNamed performs range, rate-of-change, and plausibility validation.
type ForgeCallNamed struct {
}

// NewForgeCallNamed returns an initialized validator. Stateful imports
// whose Go zero-value is not a valid initial state (e.g. filter — its
// internal pointer to the runtime implementation is nil after zero-init
// and the first Update call would deref nil) opt in to an explicit
// factory call via `ImportContext::go_init_expr`. Codec imports leave
// their slot zero-initialized (the pure-data struct's natural empty
// state). Validators with no stateful imports get an empty literal,
// matching the legacy `var v X` zero-value path callers used before
// this constructor existed.
func NewForgeCallNamed() *ForgeCallNamed {
	return &ForgeCallNamed{
	}
}

// Validate checks all validation rules and returns the result.
func (sceSelf *ForgeCallNamed) Validate(name string) ValidationResult {
	if !(name != "") {
		return ValidationResult{Valid: false, Reason: "plausibility_failed"}
	}
	return ValidationResult{Valid: true, Reason: ""}
}
