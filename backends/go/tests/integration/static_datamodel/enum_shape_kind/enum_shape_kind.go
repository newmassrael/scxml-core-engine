// SCE-GENERATED — DO NOT EDIT
// source-hash: bfc186601db587ca37c93df75a2683e7ba8b863c255844424d95ac86e99a7d9d
// source: enum_shape_kind.scxml
// SCE-MAP: enum_shape_kind.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package enum_shape_kind

import "github.com/newmassrael/sce-forge-runtime/optional"

type EnumShapeKind uint8

const (
	EnumShapeKindDot EnumShapeKind = 0
	EnumShapeKindCircle EnumShapeKind = 1
	EnumShapeKindDisc EnumShapeKind = 2
	EnumShapeKindRect EnumShapeKind = 3
	EnumShapeKindLabel EnumShapeKind = 4
	EnumShapeKindPath EnumShapeKind = 5
)

// ToUnderlying returns the carrier value this holds.
func (v EnumShapeKind) ToUnderlying() uint8 {
	return uint8(v)
}

// DeclaredName returns the name the document declares for this value, and the
// absent value for a value no variant names — one the type can hold whether or
// not its set is open, since a defined integer type holds every carrier value.
// It is an optional value as every generated answer that may be absent is
// (docs/adr/0055, section 5).
func (v EnumShapeKind) DeclaredName() optional.Of[string] {
	switch v {
	case EnumShapeKindDot:
		return optional.Some("dot")
	case EnumShapeKindCircle:
		return optional.Some("circle")
	case EnumShapeKindDisc:
		return optional.Some("disc")
	case EnumShapeKindRect:
		return optional.Some("rect")
	case EnumShapeKindLabel:
		return optional.Some("label")
	case EnumShapeKindPath:
		return optional.Some("path")
	}
	return optional.None[string]()
}

// EnumShapeKindFromUnderlying returns the variant raw declares, and the absent
// value when raw declares none — the declared set is closed.
func EnumShapeKindFromUnderlying(raw uint8) optional.Of[EnumShapeKind] {
	switch raw {
	case 0:
		return optional.Some(EnumShapeKindDot)
	case 1:
		return optional.Some(EnumShapeKindCircle)
	case 2:
		return optional.Some(EnumShapeKindDisc)
	case 3:
		return optional.Some(EnumShapeKindRect)
	case 4:
		return optional.Some(EnumShapeKindLabel)
	case 5:
		return optional.Some(EnumShapeKindPath)
	}
	return optional.None[EnumShapeKind]()
}
