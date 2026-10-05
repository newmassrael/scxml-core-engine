// SCE-MAP: filter_low_pass:1 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Runtime: sce_forge_runtime
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

func (sceSelf *FilterLowPass) Update(rawSignal float64) float64 {
	return sceSelf.impl.Update(rawSignal)
}

func (sceSelf *FilterLowPass) Reset() {
	sceSelf.impl.Reset()
}
