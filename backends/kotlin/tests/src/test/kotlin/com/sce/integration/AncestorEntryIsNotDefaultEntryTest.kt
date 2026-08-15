// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.3 + Appendix D: a compound state entered only because the target
// lies inside it does not take its default initial child — Kotlin AOT path.
//
// Appendix D asks two different questions with two functions.
// `addDescendantStatesToEnter` gives a compound state its default child and is
// called for the transition's TARGET; `addAncestorStatesToEnter` walks the
// states between the target and the LCCA and adds them WITHOUT defaults. An
// engine with one entry function answers both with the first, and two children
// of one compound state end up active at once.
//
// Measured 2026-08-15 on the worked example `examples/ai_loop/ai_loop.scxml`,
// where the wrongly-entered state's `<onentry>` sends a prompt: the supervised
// session was re-sent its opening prompt every time a person answered a dialog.
//
// Fixture: integration_resources/ancestor_entry_is_not_default_entry/ancestor_entry_is_not_default_entry.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_ancestor_entry_is_not_default_entry_kotlin.sh

package com.sce.integration

import com.sce.integration.ancestor_entry_is_not_default_entry.AncestorEntryIsNotDefaultEntryEvent
import com.sce.integration.ancestor_entry_is_not_default_entry.AncestorEntryIsNotDefaultEntryState
import com.sce.integration.ancestor_entry_is_not_default_entry.AncestorEntryIsNotDefaultEntryStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 3.3 — an ancestor on the entry chain takes no default child.
@DisplayName("AncestorEntryIsNotDefaultEntry — W3C SCXML 3.3 + Appendix D")
class AncestorEntryIsNotDefaultEntryTest {

    @Test
    fun anAncestorEnteredOnTheWayToATargetTakesNoDefaultChild() {
        // The fixture counts entries with `<assign>`, so this is an
        // ECMAScript-datamodel machine.
        val sm = AncestorEntryIsNotDefaultEntryStateMachine(W3CTestBase.createEngine())
        sm.initialize()

        // No assertion on the intermediate configurations here, and the reason
        // is this channel's shape rather than an oversight: `currentState` is a
        // single leaf, and in a `<parallel>` machine it rests wherever the last
        // region was entered. The document carries every clause instead.
        //
        // `cross` enters the `<parallel>` itself, so `run` is a parallel
        // ancestor and `drive`/`outer` are compound ones; `back` then `again`
        // repeat the entry with the parallel already active, which is a
        // different branch of the entry walk and the one a running machine
        // takes.
        sm.send(AncestorEntryIsNotDefaultEntryEvent.Cross)
        sm.tick()
        sm.send(AncestorEntryIsNotDefaultEntryEvent.Back)
        sm.tick()
        sm.send(AncestorEntryIsNotDefaultEntryEvent.Again)
        sm.tick()
        sm.send(AncestorEntryIsNotDefaultEntryEvent.Check)
        sm.tick()

        // The document checks its four clauses in document order and lands each
        // in a `<final>` of its own, so the state reported here names which one
        // broke rather than saying only "not settled".
        assertEquals(
            AncestorEntryIsNotDefaultEntryState.Settled,
            sm.currentState.value,
            "`check` did not carry the machine to `settled`: `failDefaulted` is a default " +
                "child nobody targeted, `failLobbied` is `drive`'s default taken while it " +
                "was only an ancestor, `failIdled` is the untouched region not getting its " +
                "default, `failTargeted` is a pass that never reached the target",
        )
    }
}
