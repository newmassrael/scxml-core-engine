// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Do not edit — regenerate from the source SCXML file.

class FilterMovingAverage {
    private val buffer = DoubleArray(5)
    private var index = 0
    private var filled = false

    fun update(rawTemp: Double): Double {
        buffer[index] = rawTemp.toDouble()
        index = (index + 1) % 5
        if (!filled && index == 0) filled = true
        val count = if (filled) 5 else index
        var sum: Double = 0.0
        for (i in 0 until count) sum += buffer[i]
        return sum / count.toDouble()
    }

    fun reset() {
        buffer.fill(0.0)
        index = 0
        filled = false
    }
}
