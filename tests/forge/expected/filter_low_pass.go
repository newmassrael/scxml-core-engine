// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Do not edit — regenerate from the source SCXML file.

package filter_low_pass

type FilterLowPass struct {
	prev        float64
	initialized bool
}

func (f *FilterLowPass) Update(rawSignal float64) float64 {
	if !f.initialized {
		f.prev = float64(rawSignal)
		f.initialized = true
		return f.prev
	}
	f.prev = 0.1 * float64(rawSignal) + (1.0 - 0.1) * f.prev
	return f.prev
}

func (f *FilterLowPass) Reset() {
	f.prev = 0
	f.initialized = false
}
