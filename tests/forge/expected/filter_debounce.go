// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Do not edit — regenerate from the source SCXML file.

package filter_debounce

type FilterDebounce struct {
	stableValue bool
	candidate   bool
	count       int
	initialized bool
}

func (f *FilterDebounce) Update(rawButton bool) bool {
	value := rawButton
	if !f.initialized {
		f.stableValue = value
		f.candidate = value
		f.count = 1
		f.initialized = true
		return f.stableValue
	}
	if value == f.candidate {
		f.count++
		if f.count >= 3 {
			f.stableValue = f.candidate
		}
	} else {
		f.candidate = value
		f.count = 1
	}
	return f.stableValue
}

func (f *FilterDebounce) Reset() {
	f.stableValue = false
	f.candidate = false
	f.count = 0
	f.initialized = false
}
