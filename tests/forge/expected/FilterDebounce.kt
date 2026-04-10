// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Do not edit — regenerate from the source SCXML file.

class FilterDebounce {
    private var stableValue: Boolean = false
    private var candidate: Boolean = false
    private var count = 0
    private var initialized = false

    fun update(rawButton: Boolean): Boolean {
        val value = rawButton
        if (!initialized) {
            stableValue = value
            candidate = value
            count = 1
            initialized = true
            return stableValue
        }
        if (value == candidate) {
            count++
            if (count >= 3) {
                stableValue = candidate
            }
        } else {
            candidate = value
            count = 1
        }
        return stableValue
    }

    fun reset() {
        stableValue = false
        candidate = false
        count = 0
        initialized = false
    }
}
