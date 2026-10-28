// SCE-GENERATED — DO NOT EDIT
// source-hash: b67ac92c47dabab09bc66be64b0a4b4a6acfabe17f95412102ed640a717fc04f
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
