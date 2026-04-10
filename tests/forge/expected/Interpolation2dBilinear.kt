// SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
// Do not edit — regenerate from the source SCXML file.

object Interpolation2dBilinear {
    private val AXIS_RPM = doubleArrayOf(800.0, 1200.0, 2000.0, 3000.0)
    private val AXIS_LOAD = doubleArrayOf(10.0, 50.0, 100.0)
    private val VALUES = arrayOf(
        doubleArrayOf(2.1, 4.5, 7.0),
        doubleArrayOf(2.5, 5.0, 8.0),
        doubleArrayOf(3.0, 6.0, 9.5),
        doubleArrayOf(3.5, 7.0, 11.0)
    )

    fun lookup(rpm: UShort, load: UByte): Double {
        return bilinearInterpolate(
            AXIS_RPM, AXIS_LOAD, VALUES,
            rpm.toDouble(), load.toDouble())
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

    private fun bilinearInterpolate(
            axisX: DoubleArray, axisY: DoubleArray, table: Array<DoubleArray>,
            xIn: Double, yIn: Double): Double {
        var x = xIn; var y = yIn
        if (x <= axisX[0]) x = axisX[0] else if (x >= axisX[axisX.size - 1]) x = axisX[axisX.size - 1]
        if (y <= axisY[0]) y = axisY[0] else if (y >= axisY[axisY.size - 1]) y = axisY[axisY.size - 1]
        var ix = 0; var iy = 0
        for (i in 0 until axisX.size - 1) { if (x <= axisX[i + 1]) { ix = i; break }; ix = i }
        for (i in 0 until axisY.size - 1) { if (y <= axisY[i + 1]) { iy = i; break }; iy = i }
        val tx = (x - axisX[ix]) / (axisX[ix + 1] - axisX[ix])
        val ty = (y - axisY[iy]) / (axisY[iy + 1] - axisY[iy])
        val a = table[ix][iy] + tx * (table[ix + 1][iy] - table[ix][iy])
        val b = table[ix][iy + 1] + tx * (table[ix + 1][iy + 1] - table[ix][iy + 1])
        return a + ty * (b - a)
    }
}