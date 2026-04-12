// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.filter_low_pass

import com.sce.forge.runtime.LowPass

class FilterLowPass {
    private val impl = LowPass(alpha = 0.1)

    fun update(rawSignal: Double): Double {
        return impl.update(rawSignal)
    }

    fun reset() {
        impl.reset()
    }
}
