// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Do not edit — regenerate from the source SCXML file.

package filter_moving_average

type FilterMovingAverage struct {
	buffer  [5]float64
	index   int
	filled  bool
}

func (f *FilterMovingAverage) Update(rawTemp float64) float64 {
	f.buffer[f.index] = float64(rawTemp)
	f.index = (f.index + 1) % 5
	if !f.filled && f.index == 0 {
		f.filled = true
	}
	count := 5
	if !f.filled {
		count = f.index
	}
	var sum float64
	for i := 0; i < count; i++ {
		sum += f.buffer[i]
	}
	return sum / float64(count)
}

func (f *FilterMovingAverage) Reset() {
	f.buffer = [5]float64{}
	f.index = 0
	f.filled = false
}
