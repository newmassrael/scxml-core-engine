// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package com.sce.forge.runtime

/**
 * Three signal-filter classes matching SCE_FORGE.md Section 4.8.
 *
 * `MovingAverage` and `LowPass` are fixed to `Double` to avoid JVM
 * generic-erasure boxing in the hot path. `Debounce` is generic over any
 * `T` with value semantics (Boolean, Int, String, ...).
 */

/** Sliding-window arithmetic mean. */
public class MovingAverage(private val window: Int) {
    init {
        require(window >= 1) { "window must be >= 1" }
    }

    private val buffer = DoubleArray(window)
    private var index = 0
    private var filled = false

    public fun update(value: Double): Double {
        buffer[index] = value
        index = (index + 1) % window
        if (!filled && index == 0) filled = true
        val count = if (filled) window else index
        var sum = 0.0
        for (i in 0 until count) sum += buffer[i]
        return sum / count.toDouble()
    }

    public fun reset() {
        buffer.fill(0.0)
        index = 0
        filled = false
    }
}

/**
 * First-order exponential low-pass: y[n] = alpha * x[n] + (1 - alpha) * y[n-1].
 * On the first sample, y[0] = x[0] (no warm-up bias toward zero).
 */
public class LowPass(private val alpha: Double) {
    private var state = 0.0
    private var initialized = false

    public fun update(value: Double): Double {
        if (!initialized) {
            state = value
            initialized = true
        } else {
            state = alpha * value + (1.0 - alpha) * state
        }
        return state
    }

    public fun reset() {
        state = 0.0
        initialized = false
    }
}

/**
 * Output latches to a new value only after `window` consecutive identical
 * samples. Until the buffer fills, the most recent input passes through.
 *
 * Backed by a `MutableList<T?>` so the type parameter is preserved through the
 * buffer (no unchecked casts, no raw `Any` array). The first ring slot starts
 * as `null` and is overwritten on the first `update` call; the algorithm only
 * inspects the entire window once `filled` is true, by which point every slot
 * holds a real `T`.
 */
public class Debounce<T : Any>(private val window: Int) {
    init {
        require(window >= 1) { "window must be >= 1" }
    }

    private val buffer: MutableList<T?> = MutableList(window) { null }
    private var index = 0
    private var filled = false
    private var output: T? = null

    public fun update(value: T): T {
        buffer[index] = value
        index = (index + 1) % window
        if (!filled && index == 0) filled = true

        if (filled) {
            val first = buffer[0]
            var stable = true
            for (i in 1 until window) {
                if (buffer[i] != first) {
                    stable = false
                    break
                }
            }
            if (stable) output = first
        } else {
            output = value
        }
        // `output` is set on every call (either `value` while warming up, or
        // the latched value once the window has agreed once), so the
        // not-null assertion is always satisfied at this point.
        return output!!
    }

    public fun reset() {
        for (i in 0 until window) buffer[i] = null
        index = 0
        filled = false
        output = null
    }
}
