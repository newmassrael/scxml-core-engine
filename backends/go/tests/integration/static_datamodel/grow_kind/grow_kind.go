// SCE-GENERATED — DO NOT EDIT
// source-hash: be25ee8255a32c7ec240ba31f48b3e3888b421310628dd926f72cb5a0da29c66
// source: grow_kind.scxml
// SCE-MAP: grow_kind.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package grow_kind

import "github.com/newmassrael/sce-forge-runtime/optional"

type GrowKind uint8

const (
	GrowKindSingle GrowKind = 1
	GrowKindNone GrowKind = 2
)

// ToUnderlying returns the carrier value this holds.
func (v GrowKind) ToUnderlying() uint8 {
	return uint8(v)
}

// DeclaredName returns the name the document declares for this value, and the
// absent value for a value no variant names — one the type can hold whether or
// not its set is open, since a defined integer type holds every carrier value.
// It is an optional value as every generated answer that may be absent is
// (docs/adr/0055, section 5).
func (v GrowKind) DeclaredName() optional.Of[string] {
	switch v {
	case GrowKindSingle:
		return optional.Some("single")
	case GrowKindNone:
		return optional.Some("none")
	}
	return optional.None[string]()
}

// GrowKindFromUnderlying returns the variant raw declares, and the absent
// value when raw declares none — the declared set is closed.
func GrowKindFromUnderlying(raw uint8) optional.Of[GrowKind] {
	switch raw {
	case 1:
		return optional.Some(GrowKindSingle)
	case 2:
		return optional.Some(GrowKindNone)
	}
	return optional.None[GrowKind]()
}
