// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1 + C.1 (docs/adr/0090, test 1): the origin of an event the
// SCXML Event I/O Processor delivers is the address its sender published, on a
// self-send, a child's send to its parent and a parent's send to its child, and
// a reply to it arrives — Kotlin AOT path.
//
// The runtime writes the pair where it builds the event (`EventMetadata.sentBy`
// at the generated send site, the parent's `onSendToParent`, `handToChild`), a
// self-send written with the alias `type="scxml"` still reads the processor's
// URI, and a child reaches its parent at the parent's published location
// (`isParentSession`). The child's `hi` used to read the child's own location,
// which the binding filled in for an event with none, so its reply went back to
// itself and the parent waited.
//
// Fixture:
// integration_resources/an_origin_is_the_address_its_sender_published/an_origin_is_the_address_its_sender_published.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/materialize.sh kotlin-integration

package com.sce.integration

import com.sce.integration.an_origin_is_the_address_its_sender_published.AnOriginIsTheAddressItsSenderPublishedState
import com.sce.integration.an_origin_is_the_address_its_sender_published.AnOriginIsTheAddressItsSenderPublishedStateMachine
import com.sce.w3c.W3CTestBase
import com.sce.w3c.driveOnVirtualTime
import com.sce.w3c.initializeOnVirtualTime
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("AnOriginIsTheAddressItsSenderPublished — W3C SCXML 5.10.1, C.1")
class AnOriginIsTheAddressItsSenderPublishedTest {

    @Test
    fun anOriginIsTheAddressItsSenderPublished() {
        val sm = AnOriginIsTheAddressItsSenderPublishedStateMachine(W3CTestBase.createEngine())
        try {
            sm.initializeOnVirtualTime()
            sm.driveOnVirtualTime(2000L)

            assertEquals(
                AnOriginIsTheAddressItsSenderPublishedState.Done,
                sm.terminalState,
                "the run must end in `done`; parked in ${sm.currentState.value}: a reply sent to " +
                    "`_event.origin` with `_event.origintype` reached nobody",
            )
            val observed = mapOf(
                "selfOk" to (sm.selfOk() to 1L),
                "selfReplied" to (sm.selfReplied() to 1L),
                "childOk" to (sm.childOk() to 1L),
                "childReplied" to (sm.childReplied() to 1L),
                "parentOk" to (sm.parentOk() to 1L),
                "parentReplied" to (sm.parentReplied() to 1L),
            )
            val wrong = observed.filterValues { (got, want) -> got != want }
            assertEquals(emptyMap<String, Pair<Long?, Long>>(), wrong, "observed (got to want)")
        } finally {
            sm.cleanup()
        }
    }
}
