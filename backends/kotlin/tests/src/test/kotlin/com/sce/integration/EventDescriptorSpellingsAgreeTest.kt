// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.12.1: `wild`, `wild.` and `wild.*` are one descriptor — Kotlin AOT path.
//
// The clause calls the three spellings "functionally equivalent since they are
// token prefixes of exactly the same set of event names", and a descriptor
// ending in `.*` matches "zero or more tokens", so a bare `.*` is an empty
// token prefix and matches every event.
//
// No W3C fixture delivers a bare `foo` to a `foo.*` handler, and the four that
// write a bare `.*` write it as a catch-all to fail — which an engine that
// matches nothing on `.*` passes by never taking the transition it must not
// take. This fixture puts every case in positive polarity instead.
//
// Fixture: integration_resources/event_descriptor_spellings_agree/event_descriptor_spellings_agree.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_event_descriptor_spellings_agree_kotlin.sh

package com.sce.integration

import com.sce.integration.event_descriptor_spellings_agree.EventDescriptorSpellingsAgreeState
import com.sce.integration.event_descriptor_spellings_agree.EventDescriptorSpellingsAgreeStateMachine
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 3.12.1 — every spelling of one event descriptor matches the same events (Kotlin AOT).
@DisplayName("EventDescriptorSpellingsAgree — W3C SCXML 3.12.1")
class EventDescriptorSpellingsAgreeTest {

    @Test
    fun everySpellingOfOneDescriptorCatchesTheSameEvents() {
        val sm = EventDescriptorSpellingsAgreeStateMachine()
        sm.initialize()

        val deadline = System.currentTimeMillis() + 2000L
        while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
            sm.tick()
            Thread.sleep(10)
        }

        // Each failure final names the case, so this one assertion says which
        // spelling disagreed with the clause:
        //   FailSuffixed   `wild.*` did not catch bare `wild`
        //   FailDotted     `dot.` did not catch bare `dot`
        //   FailBounded    `wild.*` caught `wilder`, across a token boundary
        //   FailUniversal  a bare `.*` did not catch `any.token.sequence`
        assertEquals(
            EventDescriptorSpellingsAgreeState.Pass,
            sm.terminalState,
            "the machine rested in a failure final: `wild.*` must catch bare `wild` " +
                "and `dot.` must catch bare `dot` (the clause calls the spellings " +
                "functionally equivalent), a bare `.*` must catch every event, and " +
                "`wild.*` must NOT catch `wilder` because the prefix is a whole token"
        )
    }
}
