// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Child sessions" — Kotlin half of the Rust suite's
// `a_static_child_string_is_held_to_its_bound.rs`.
//
// A string an `<invoke type="scxml">` hands its child is held to the bound the
// child declared for that variable, in UTF-8 bytes, as an `<assign>` to it would
// be. A value past it is the evaluation that failed (W3C SCXML 5.7.1):
// `error.execution` is raised, that one value is left out, and the child still
// starts, holding the one its `<data>` gave it.
//
// `static_invoke_string.scxml` invokes three children whose `title` holds four
// bytes: `fits` is handed 'wxyz' and ends on it; `over` is handed eight bytes and
// `wide` two characters of five bytes, both past the bound, so each starts with
// the 'ab' its `<data>` gave it, which it ends on. Kotlin counts a string in
// UTF-16 units by default, so `wide` is the case that tells the bound is in bytes.

package com.sce.integration

import com.sce.integration.static_invoke_string.StaticInvokeStringStateMachine
import com.sce.runtime.ManualClock
import kotlin.test.Test
import kotlin.test.assertEquals

class AStaticChildStringIsHeldToItsBoundTest {

    @Test
    fun aStringPastTheChildsBoundIsLeftOutAndReported() {
        val sm = StaticInvokeStringStateMachine()
        sm.clock = ManualClock(0)
        sm.initialize()
        try {
            // Let the children run and report to their parent.
            repeat(5) { sm.tick() }
            // 1 (`fits`) + 10 (`over`) + 100 (`wide`): all three ended, so none
            // held a value past its bound but the one that fits.
            assertEquals(111u, sm.completed)
            // The two values past the bound were reported, once each.
            assertEquals(2u, sm.errors)
        } finally {
            sm.cleanup()
        }
    }
}
