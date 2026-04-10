// SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
// Do not edit — regenerate from the source SCXML file.

import com.sce.forge.runtime.bilinear

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
        return bilinear(
            AXIS_RPM, AXIS_LOAD, VALUES,
            rpm.toDouble(), load.toDouble()
        )
    }
}