// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Do not edit — regenerate from the source SCXML file.

class FilterLowPass {
    private var prev: Double = 0.0
    private var initialized = false

    fun update(rawSignal: Double): Double {
        if (!initialized) {
            prev = rawSignal.toDouble()
            initialized = true
            return prev
        }
        prev = 0.1.toDouble() * rawSignal.toDouble() + (1.0 - 0.1).toDouble() * prev
        return prev
    }

    fun reset() {
        prev = 0.0
        initialized = false
    }
}
