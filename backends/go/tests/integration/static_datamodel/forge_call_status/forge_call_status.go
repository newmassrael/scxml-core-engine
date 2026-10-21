// SCE-GENERATED — DO NOT EDIT
// source-hash: 048fe3b13b4ba344e3f60c0d3a93bf8f8568ab7bd254b4d4763d7e631895e2f6
// SCE-MAP: forge_call_status.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package forge_call_status

type ForgeCallStatus uint8

const (
	ForgeCallStatusOpen ForgeCallStatus = 1
	ForgeCallStatusHeld ForgeCallStatus = 2
	ForgeCallStatusDone ForgeCallStatus = 3
)

// ToUnderlying returns the carrier value this holds.
func (v ForgeCallStatus) ToUnderlying() uint8 {
	return uint8(v)
}

// DeclaredName returns the name the document declares for this value, and
// false for a value no variant names — one the type can hold whether or not
// its set is open, since a defined integer type holds every carrier value.
func (v ForgeCallStatus) DeclaredName() (string, bool) {
	switch v {
	case ForgeCallStatusOpen:
		return "open", true
	case ForgeCallStatusHeld:
		return "held", true
	case ForgeCallStatusDone:
		return "done", true
	}
	return "", false
}

// ForgeCallStatusFromUnderlying returns the variant raw declares, and false
// when raw declares none — the declared set is closed.
func ForgeCallStatusFromUnderlying(raw uint8) (ForgeCallStatus, bool) {
	switch raw {
	case 1:
		return ForgeCallStatusOpen, true
	case 2:
		return ForgeCallStatusHeld, true
	case 3:
		return ForgeCallStatusDone, true
	}
	return 0, false
}
