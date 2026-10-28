// SCE-GENERATED — DO NOT EDIT
// source-hash: b67ac92c47dabab09bc66be64b0a4b4a6acfabe17f95412102ed640a717fc04f
// SCE-MAP: forge_call_level.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package forge_call_level

type ForgeCallLevel uint8

const (
	ForgeCallLevelLight ForgeCallLevel = 10
	ForgeCallLevelHeavy ForgeCallLevel = 20
)

// ToUnderlying returns the carrier value this holds.
func (v ForgeCallLevel) ToUnderlying() uint8 {
	return uint8(v)
}

// DeclaredName returns the name the document declares for this value, and
// false for a value no variant names — one the type can hold whether or not
// its set is open, since a defined integer type holds every carrier value.
func (v ForgeCallLevel) DeclaredName() (string, bool) {
	switch v {
	case ForgeCallLevelLight:
		return "light", true
	case ForgeCallLevelHeavy:
		return "heavy", true
	}
	return "", false
}

// ForgeCallLevelFromUnderlying returns the variant raw declares, and false
// when raw declares none — the declared set is closed.
func ForgeCallLevelFromUnderlying(raw uint8) (ForgeCallLevel, bool) {
	switch raw {
	case 10:
		return ForgeCallLevelLight, true
	case 20:
		return ForgeCallLevelHeavy, true
	}
	return 0, false
}
