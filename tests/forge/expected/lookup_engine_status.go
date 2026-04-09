// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Do not edit — regenerate from the source SCXML file.

package lookup_engine_status

// Status represents the lookup output values.
type Status int

const (
	StatusSTOP Status = iota
	StatusRUNNING
	StatusFAULT
)

// LookupStatus maps engSta to its corresponding Status.
func LookupStatus(engSta uint8) Status {
	switch engSta {
	case 0x07:
		return StatusFAULT
	case 0x03:
		return StatusRUNNING
	case 0x00, 0x01, 0x02:
		return StatusSTOP
	default:
		return StatusSTOP
	}
}