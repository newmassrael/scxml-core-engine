// SCE-MAP: transform_bitwise:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package transform_bitwise

// ComputeHighNibble computes the highNibble output.
func ComputeHighNibble(byte_ uint8) uint8 {
	return byte_ >> 4 & 0x0F
}

// ComputeLowNibble computes the lowNibble output.
func ComputeLowNibble(byte_ uint8) uint8 {
	return byte_ & 0x0F
}
