// SCE-GENERATED — DO NOT EDIT
// source-hash: db4336c6a9bf789bb8f0ab985d2bf7ad97273b841d99ca0c54e9bd1050729ae4
// SCE-MAP: shape_kind.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package shape_kind

import "github.com/newmassrael/sce-forge-runtime/optional"

type ShapeKind uint8

const (
	ShapeKindDot ShapeKind = 0
	ShapeKindCircle ShapeKind = 1
	ShapeKindRect ShapeKind = 2
)

// ToUnderlying returns the carrier value this holds.
func (v ShapeKind) ToUnderlying() uint8 {
	return uint8(v)
}

// DeclaredName returns the name the document declares for this value, and the
// absent value for a value no variant names — one the type can hold whether or
// not its set is open, since a defined integer type holds every carrier value.
// It is an optional value as every generated answer that may be absent is
// (docs/adr/0055, section 5).
func (v ShapeKind) DeclaredName() optional.Of[string] {
	switch v {
	case ShapeKindDot:
		return optional.Some("dot")
	case ShapeKindCircle:
		return optional.Some("circle")
	case ShapeKindRect:
		return optional.Some("rect")
	}
	return optional.None[string]()
}

// ShapeKindFromUnderlying returns the variant raw declares, and the absent
// value when raw declares none — the declared set is closed.
func ShapeKindFromUnderlying(raw uint8) optional.Of[ShapeKind] {
	switch raw {
	case 0:
		return optional.Some(ShapeKindDot)
	case 1:
		return optional.Some(ShapeKindCircle)
	case 2:
		return optional.Some(ShapeKindRect)
	}
	return optional.None[ShapeKind]()
}
