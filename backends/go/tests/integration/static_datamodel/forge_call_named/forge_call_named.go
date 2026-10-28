// SCE-GENERATED — DO NOT EDIT
// source-hash: d11fb6db69c27fed25f47a2f22a5bf7fb78da524d08f0c299e9cc553d32a6375
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
