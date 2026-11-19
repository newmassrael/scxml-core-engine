// SCE-GENERATED — DO NOT EDIT
// source-hash: 18928854374741857115357dfd562d36b7bfe0eb18c6ae5bab5a940a32fc8f71
// source: enum_signal_level.scxml
// SCE-MAP: enum_signal_level.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package enum_signal_level

import "github.com/newmassrael/sce-forge-runtime/optional"

type EnumSignalLevel uint8

const (
	EnumSignalLevelLow EnumSignalLevel = 0
	EnumSignalLevelHigh EnumSignalLevel = 1
	EnumSignalLevelCritical EnumSignalLevel = 2
)

// ToUnderlying returns the carrier value this holds.
func (v EnumSignalLevel) ToUnderlying() uint8 {
	return uint8(v)
}

// DeclaredName returns the name the document declares for this value, and the
// absent value for a value no variant names — one the type can hold whether or
// not its set is open, since a defined integer type holds every carrier value.
// It is an optional value as every generated answer that may be absent is
// (docs/adr/0055, section 5).
func (v EnumSignalLevel) DeclaredName() optional.Of[string] {
	switch v {
	case EnumSignalLevelLow:
		return optional.Some("low")
	case EnumSignalLevelHigh:
		return optional.Some("high")
	case EnumSignalLevelCritical:
		return optional.Some("critical")
	}
	return optional.None[string]()
}

// EnumSignalLevelFromUnderlying returns the variant raw declares, and the absent
// value when raw declares none — the declared set is closed.
func EnumSignalLevelFromUnderlying(raw uint8) optional.Of[EnumSignalLevel] {
	switch raw {
	case 0:
		return optional.Some(EnumSignalLevelLow)
	case 1:
		return optional.Some(EnumSignalLevelHigh)
	case 2:
		return optional.Some(EnumSignalLevelCritical)
	}
	return optional.None[EnumSignalLevel]()
}
