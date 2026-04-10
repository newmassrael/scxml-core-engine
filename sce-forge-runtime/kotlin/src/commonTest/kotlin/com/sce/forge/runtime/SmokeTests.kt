// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package com.sce.forge.runtime

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

/**
 * Sanity tests for sce_forge_runtime. These verify that the algorithms produce
 * the same numerical results as the C++/Rust/Python implementations on the
 * same inputs (de-facto cross-language conformance until a dedicated harness
 * exists), and that the HAL interfaces enforce their contracts at compile
 * time.
 */
class SmokeTests {

    @Test
    fun linearMidpoint() {
        val axis = doubleArrayOf(0.0, 1.0, 2.0, 3.0)
        val vals = doubleArrayOf(10.0, 20.0, 30.0, 40.0)
        assertEquals(25.0, linear(axis, vals, 1.5), 1e-12)
    }

    @Test
    fun bilinearCentre() {
        val ax = doubleArrayOf(0.0, 1.0)
        val ay = doubleArrayOf(0.0, 1.0)
        val tab = arrayOf(
            doubleArrayOf(0.0, 1.0),
            doubleArrayOf(2.0, 3.0),
        )
        assertEquals(1.5, bilinear(ax, ay, tab, 0.5, 0.5), 1e-12)
    }

    @Test
    fun movingAverageFirstSample() {
        val ma = MovingAverage(window = 4)
        assertEquals(1.0, ma.update(1.0), 1e-12)
    }

    @Test
    fun lowPassFirstSamplePassesThrough() {
        val lp = LowPass(alpha = 0.1)
        assertEquals(7.0, lp.update(7.0), 1e-12)
    }

    @Test
    fun debounceUntilFilled() {
        val db = Debounce<Boolean>(window = 3)
        assertEquals(true, db.update(true))
        assertEquals(false, db.update(false))
    }

    private class DummyTimer : Timer {
        override fun startPeriodic(intervalMs: Long, callback: () -> Unit) {}
        override fun startOneShot(delayMs: Long, callback: () -> Unit) {}
        override fun cancel() {}
    }

    @Test
    fun timerInterfaceIsImplementable() {
        val t: Timer = DummyTimer()
        t.cancel()
    }

    private enum class AlertTag { LOW, HIGH }
    private class AlertDomain : EventDomain<AlertTag>

    @Test
    fun thresholdStateTransitions() {
        val st = ThresholdState()
        assertTrue(st.enterIf(true))
        assertFalse(st.enterIf(true))
        assertTrue(st.leaveIf(true))
    }

    @Test
    fun eventQueuePushAndIterate() {
        val q = EventQueue<AlertTag>()
        assertTrue(q.isEmpty())
        q.push(AlertTag.HIGH)
        q.push(AlertTag.LOW)
        assertEquals(2, q.size)
        assertEquals(AlertTag.HIGH, q[0])
        assertEquals(AlertTag.LOW, q[1])
    }
}
