// SCE-MAP: filter_debounce:1

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.filter_debounce

import com.sce.forge.runtime.Debounce

class FilterDebounce {
    private val impl = Debounce<Boolean>(window = 3)

    fun update(rawButton: Boolean): Boolean {
        return impl.update(rawButton)
    }

    fun reset() {
        impl.reset()
    }
}
