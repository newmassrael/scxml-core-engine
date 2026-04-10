// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.filter_moving_average

import com.sce.forge.runtime.MovingAverage

class FilterMovingAverage {
    private val impl = MovingAverage(window = 5)

    fun update(rawTemp: Double): Double {
        return impl.update(rawTemp)
    }

    fun reset() {
        impl.reset()
    }
}