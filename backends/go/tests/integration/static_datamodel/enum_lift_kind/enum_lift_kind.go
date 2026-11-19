// SCE-GENERATED — DO NOT EDIT
// source-hash: 257fc4ff0ee49e833cae625af6f30d3cba25016aee82a12b9a36aa0f4666b33d
// source: enum_lift_kind.scxml
// SCE-MAP: enum_lift_kind.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package enum_lift_kind

import "github.com/newmassrael/sce-forge-runtime/optional"

type EnumLiftKind uint8

const (
	EnumLiftKindMeeting EnumLiftKind = 0
	EnumLiftKindTask EnumLiftKind = 1
)

// ToUnderlying returns the carrier value this holds.
func (v EnumLiftKind) ToUnderlying() uint8 {
	return uint8(v)
}

// DeclaredName returns the name the document declares for this value, and the
// absent value for a value no variant names — one the type can hold whether or
// not its set is open, since a defined integer type holds every carrier value.
// It is an optional value as every generated answer that may be absent is
// (docs/adr/0055, section 5).
func (v EnumLiftKind) DeclaredName() optional.Of[string] {
	switch v {
	case EnumLiftKindMeeting:
		return optional.Some("meeting")
	case EnumLiftKindTask:
		return optional.Some("task")
	}
	return optional.None[string]()
}

// EnumLiftKindFromUnderlying returns the variant raw declares, and the absent
// value when raw declares none — the declared set is closed.
func EnumLiftKindFromUnderlying(raw uint8) optional.Of[EnumLiftKind] {
	switch raw {
	case 0:
		return optional.Some(EnumLiftKindMeeting)
	case 1:
		return optional.Some(EnumLiftKindTask)
	}
	return optional.None[EnumLiftKind]()
}
