// SCE-MAP: lookup_gear_position:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package lookup_gear_position

// Gear represents the lookup output values.
type Gear int

const (
	GearPARK Gear = iota
	GearREVERSE
	GearNEUTRAL
	GearDRIVE
	GearSPORT
)

// LookupGear maps gearRaw to its corresponding Gear.
func LookupGear(gearRaw uint8) Gear {
	switch gearRaw {
	case 3:
		return GearDRIVE
	case 2:
		return GearNEUTRAL
	case 0:
		return GearPARK
	case 1:
		return GearREVERSE
	case 4:
		return GearSPORT
	default:
		return GearNEUTRAL
	}
}
