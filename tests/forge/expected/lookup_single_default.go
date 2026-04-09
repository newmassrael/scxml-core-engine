// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Do not edit — regenerate from the source SCXML file.

package lookup_single_default

// Quality represents the lookup output values.
type Quality int

const (
	QualityNONE Quality = iota
	QualityLOW
	QualityMEDIUM
	QualityHIGH
)

// LookupQuality maps level to its corresponding Quality.
func LookupQuality(level uint8) Quality {
	switch level {
	case 3:
		return QualityHIGH
	case 1:
		return QualityLOW
	case 2:
		return QualityMEDIUM
	case 0:
		return QualityNONE
	default:
		return QualityNONE
	}
}