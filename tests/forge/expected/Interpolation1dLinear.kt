// SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
// Do not edit — regenerate from the source SCXML file.

object Interpolation1dLinear {
    private val AXIS_RPM = doubleArrayOf(800.0, 1200.0, 2000.0, 3000.0, 4000.0, 6000.0)
    private val VALUES = doubleArrayOf(120.0, 145.0, 200.0, 230.0, 210.0, 180.0)

    fun lookup(rpm: UShort): Double {
        return linearInterpolate(
            AXIS_RPM, VALUES,
            rpm.toDouble())
    }

    private fun linearInterpolate(axis: DoubleArray, values: DoubleArray, x: Double): Double {
        if (x <= axis[0]) return values[0]
        if (x >= axis[axis.size - 1]) return values[axis.size - 1]
        for (i in 0 until axis.size - 1) {
            if (x <= axis[i + 1]) {
                val t = (x - axis[i]) / (axis[i + 1] - axis[i])
                return values[i] + t * (values[i + 1] - values[i])
            }
        }
        return values[axis.size - 1]
    }
}