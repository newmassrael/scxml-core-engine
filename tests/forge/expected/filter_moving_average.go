// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package filter_moving_average

import "github.com/newmassrael/sce-forge-runtime/filter"

type FilterMovingAverage struct {
	impl *filter.MovingAverage[float64]
}

func NewFilterMovingAverage() *FilterMovingAverage {
	return &FilterMovingAverage{
		impl: filter.NewMovingAverage[float64](5),
	}
}

func (f *FilterMovingAverage) Update(rawTemp float64) float64 {
	return f.impl.Update(rawTemp)
}

func (f *FilterMovingAverage) Reset() {
	f.impl.Reset()
}
