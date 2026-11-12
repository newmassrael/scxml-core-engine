// SCE-GENERATED — DO NOT EDIT
// source-hash: 6d71812086a998a80412c256f457bab009adc259010083f54f89e8deddbca71a
// SCE-MAP: shape_kind_open.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package shape_kind_open

import "github.com/newmassrael/sce-forge-runtime/optional"

type ShapeKindOpen uint8

const (
	ShapeKindOpenDot ShapeKindOpen = 0
	ShapeKindOpenCircle ShapeKindOpen = 1
	ShapeKindOpenRect ShapeKindOpen = 2
)

// ToUnderlying returns the carrier value this holds.
func (v ShapeKindOpen) ToUnderlying() uint8 {
	return uint8(v)
}

// DeclaredName returns the name the document declares for this value, and the
// absent value for a value no variant names — one the type can hold whether or
// not its set is open, since a defined integer type holds every carrier value.
// It is an optional value as every generated answer that may be absent is
// (docs/adr/0055, section 5).
func (v ShapeKindOpen) DeclaredName() optional.Of[string] {
	switch v {
	case ShapeKindOpenDot:
		return optional.Some("dot")
	case ShapeKindOpenCircle:
		return optional.Some("circle")
	case ShapeKindOpenRect:
		return optional.Some("rect")
	}
	return optional.None[string]()
}

// ShapeKindOpenFromUnderlying returns the value raw stands for. Total:
// sce:strict-variants="false" leaves the declared set open, so every carrier
// value is one of this type's values.
func ShapeKindOpenFromUnderlying(raw uint8) ShapeKindOpen {
	return ShapeKindOpen(raw)
}
