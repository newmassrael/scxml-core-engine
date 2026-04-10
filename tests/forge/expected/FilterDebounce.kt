// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Do not edit — regenerate from the source SCXML file.

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