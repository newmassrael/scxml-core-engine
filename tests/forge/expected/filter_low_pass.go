// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Do not edit — regenerate from the source SCXML file.

package filter_low_pass

import "github.com/newmassrael/sce-forge-runtime/filter"

type FilterLowPass struct {
	impl *filter.LowPass[float64]
}

func NewFilterLowPass() *FilterLowPass {
	return &FilterLowPass{
		impl: filter.NewLowPass[float64](0.1),
	}
}

func (f *FilterLowPass) Update(rawSignal float64) float64 {
	return f.impl.Update(rawSignal)
}

func (f *FilterLowPass) Reset() {
	f.impl.Reset()
}
