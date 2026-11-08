// SCE-GENERATED — DO NOT EDIT
// source-hash: 9c4f25c71477b8168828f3c8f9ccad85a4253ff428db93b5b34a11bbacc1faa1
// SCE-MAP: enum_view_mode.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package enum_view_mode

import "github.com/newmassrael/sce-forge-runtime/optional"

type EnumViewMode uint8

const (
	EnumViewModeMonth EnumViewMode = 0
	EnumViewModeWeek EnumViewMode = 1
	EnumViewModeDay EnumViewMode = 2
	EnumViewModeAgendaList EnumViewMode = 3
)

// ToUnderlying returns the carrier value this holds.
func (v EnumViewMode) ToUnderlying() uint8 {
	return uint8(v)
}

// DeclaredName returns the name the document declares for this value, and the
// absent value for a value no variant names — one the type can hold whether or
// not its set is open, since a defined integer type holds every carrier value.
// It is an optional value as every generated answer that may be absent is
// (docs/adr/0055, section 5).
func (v EnumViewMode) DeclaredName() optional.Of[string] {
	switch v {
	case EnumViewModeMonth:
		return optional.Some("month")
	case EnumViewModeWeek:
		return optional.Some("week")
	case EnumViewModeDay:
		return optional.Some("day")
	case EnumViewModeAgendaList:
		return optional.Some("agenda_list")
	}
	return optional.None[string]()
}

// EnumViewModeFromUnderlying returns the variant raw declares, and the absent
// value when raw declares none — the declared set is closed.
func EnumViewModeFromUnderlying(raw uint8) optional.Of[EnumViewMode] {
	switch raw {
	case 0:
		return optional.Some(EnumViewModeMonth)
	case 1:
		return optional.Some(EnumViewModeWeek)
	case 2:
		return optional.Some(EnumViewModeDay)
	case 3:
		return optional.Some(EnumViewModeAgendaList)
	}
	return optional.None[EnumViewMode]()
}
