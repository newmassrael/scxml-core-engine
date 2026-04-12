// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package filter_debounce

import "github.com/newmassrael/sce-forge-runtime/filter"

type FilterDebounce struct {
	impl *filter.Debounce[bool]
}

func NewFilterDebounce() *FilterDebounce {
	return &FilterDebounce{
		impl: filter.NewDebounce[bool](3),
	}
}

func (f *FilterDebounce) Update(rawButton bool) bool {
	return f.impl.Update(rawButton)
}

func (f *FilterDebounce) Reset() {
	f.impl.Reset()
}
