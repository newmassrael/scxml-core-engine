// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Do not edit — regenerate from the source SCXML file.

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