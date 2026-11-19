// SCE-GENERATED — DO NOT EDIT
// source-hash: 7973737417cb8ea47734770193e07481b6487f8ce008338d6d9a4887b3c44cb5
// source: paint_color.scxml
// SCE-MAP: paint_color.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package paint_color

import "github.com/newmassrael/sce-forge-runtime/optional"

type PaintColor uint8

const (
	PaintColorRed PaintColor = 0
	PaintColorGreen PaintColor = 1
	PaintColorBlue PaintColor = 2
)

// ToUnderlying returns the carrier value this holds.
func (v PaintColor) ToUnderlying() uint8 {
	return uint8(v)
}

// DeclaredName returns the name the document declares for this value, and the
// absent value for a value no variant names — one the type can hold whether or
// not its set is open, since a defined integer type holds every carrier value.
// It is an optional value as every generated answer that may be absent is
// (docs/adr/0055, section 5).
func (v PaintColor) DeclaredName() optional.Of[string] {
	switch v {
	case PaintColorRed:
		return optional.Some("red")
	case PaintColorGreen:
		return optional.Some("green")
	case PaintColorBlue:
		return optional.Some("blue")
	}
	return optional.None[string]()
}

// PaintColorFromUnderlying returns the variant raw declares, and the absent
// value when raw declares none — the declared set is closed.
func PaintColorFromUnderlying(raw uint8) optional.Of[PaintColor] {
	switch raw {
	case 0:
		return optional.Some(PaintColorRed)
	case 1:
		return optional.Some(PaintColorGreen)
	case 2:
		return optional.Some(PaintColorBlue)
	}
	return optional.None[PaintColor]()
}
