// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.3: a typed `<data>` whose id is a Kotlin hard keyword still gets
// a reader, and it reads its own variable. Kotlin AOT.
//
// `<data id="object">` used to generate `fun object()`, which does not
// compile. `sce-build/src/reader_names.rs` now spells it `` `object` `` — the
// language's own escape — and gives no reader to an id no backend can spell
// (`auto`, `self`, `new`, `start`, `t`, and `a-b` beside `a_b`). `start` is
// the case this backend owns: a reader of that name would hide
// `StateMachineEngine.start()`.
//
// Fixture: integration_resources/typed_reader_names/typed_reader_names.scxml
// (canonical, shared with the C++ / C11 / Go / Python / Rust channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_typed_reader_names_kotlin.sh

package com.sce.integration

import com.sce.integration.typed_reader_names.TypedReaderNamesEvent
import com.sce.integration.typed_reader_names.TypedReaderNamesStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("TypedReaderNames — W3C SCXML 5.3")
class TypedReaderNamesTest {

    private fun started(): TypedReaderNamesStateMachine {
        val sm = TypedReaderNamesStateMachine(W3CTestBase.createEngine())
        sm.initialize()
        return sm
    }

    /// Each reader reads the value its own variable was declared with.
    @Test
    fun aReaderReadsItsOwnVariable() {
        val sm = started()
        try {
            assertEquals(
                listOf(1L, 2L, 3L, 4L, 10L),
                listOf(sm.box(), sm.`object`(), sm.pass(), sm.screenRules(), sm.aB())
            )
        } finally {
            sm.cleanup()
        }
    }

    /// And the value it holds now: `bump` adds 10 to four of them, and leaves
    /// `screen-rules` — which no ECMAScript expression can name — alone.
    @Test
    fun aReaderReadsTheLiveValue() {
        val sm = started()
        try {
            sm.send(TypedReaderNamesEvent.Bump)
            sm.tick()
            assertEquals(
                listOf(11L, 12L, 13L, 4L, 20L),
                listOf(sm.box(), sm.`object`(), sm.pass(), sm.screenRules(), sm.aB())
            )
        } finally {
            sm.cleanup()
        }
    }
}
