// SCE-GENERATED — DO NOT EDIT
// source-hash: b67ac92c47dabab09bc66be64b0a4b4a6acfabe17f95412102ed640a717fc04f
// SCE-MAP: forge_call_level.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package forge_call_level

import "github.com/newmassrael/sce-forge-runtime/optional"

type ForgeCallLevel uint8

const (
	ForgeCallLevelLight ForgeCallLevel = 10
	ForgeCallLevelHeavy ForgeCallLevel = 20
)

// ToUnderlying returns the carrier value this holds.
func (v ForgeCallLevel) ToUnderlying() uint8 {
	return uint8(v)
}

// DeclaredName returns the name the document declares for this value, and the
// absent value for a value no variant names — one the type can hold whether or
// not its set is open, since a defined integer type holds every carrier value.
// It is an optional value as every generated answer that may be absent is
// (docs/adr/0055, section 5).
func (v ForgeCallLevel) DeclaredName() optional.Of[string] {
	switch v {
	case ForgeCallLevelLight:
		return optional.Some("light")
	case ForgeCallLevelHeavy:
		return optional.Some("heavy")
	}
	return optional.None[string]()
}

// ForgeCallLevelFromUnderlying returns the variant raw declares, and the absent
// value when raw declares none — the declared set is closed.
func ForgeCallLevelFromUnderlying(raw uint8) optional.Of[ForgeCallLevel] {
	switch raw {
	case 10:
		return optional.Some(ForgeCallLevelLight)
	case 20:
		return optional.Some(ForgeCallLevelHeavy)
	}
	return optional.None[ForgeCallLevel]()
}
