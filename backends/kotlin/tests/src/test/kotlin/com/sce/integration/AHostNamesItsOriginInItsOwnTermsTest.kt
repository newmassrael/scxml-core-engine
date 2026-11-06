// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1 (docs/adr/0090, test 3): a host names the origin of an event
// it delivers in its own terms — an ending of a host-run invocation whose origin
// is {x-sce-host, motor} reads back `motor` and `x-sce-host` — Kotlin AOT path.
//
// The address has no scheme, so it is no SCXML location, and the type is the
// host's. The Kotlin binding used to wrap such an address as `sce://scxml/motor`
// and, for an ending that named no type, fill in the SCXML processor's;
// `completeHostInvoke` now takes the pair whole and the binding reads it as
// written.
//
// Fixture:
// integration_resources/a_host_names_its_origin_in_its_own_terms/a_host_names_its_origin_in_its_own_terms.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_a_host_names_its_origin_in_its_own_terms_kotlin.sh
// (generated with `--host-invoker x-sce-host`, the type registered below).

package com.sce.integration

import com.sce.integration.a_host_names_its_origin_in_its_own_terms.AHostNamesItsOriginInItsOwnTermsState
import com.sce.integration.a_host_names_its_origin_in_its_own_terms.AHostNamesItsOriginInItsOwnTermsStateMachine
import com.sce.runtime.EventOrigin
import com.sce.w3c.W3CTestBase
import com.sce.w3c.initializeOnVirtualTime
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("AHostNamesItsOriginInItsOwnTerms — W3C SCXML 5.10.1")
class AHostNamesItsOriginInItsOwnTermsTest {

    /** The type the regeneration script declares to the build. */
    private val hostType = "x-sce-host"

    @Test
    fun aHostNamesItsOriginInItsOwnTerms() {
        val sm = AHostNamesItsOriginInItsOwnTermsStateMachine(W3CTestBase.createEngine())
        // An invoker whose work outlives the start: the test ends it, naming the
        // party it stands for.
        val tokens = mutableListOf<Long>()
        sm.registerInvoker(hostType) { event ->
            event.start?.let { tokens.add(it.token) }
            null
        }
        try {
            sm.initializeOnVirtualTime()
            sm.tick()
            assertEquals(1, tokens.size, "the host was not asked to run `job` once")

            assertTrue(
                sm.completeHostInvoke(hostType, "job", tokens.single(), "{}", EventOrigin(hostType, "motor")),
                "the running invocation's completion was refused",
            )
            sm.tick()

            assertEquals(
                AHostNamesItsOriginInItsOwnTermsState.Done,
                sm.terminalState,
                "the run must end in `done`; parked in ${sm.currentState.value}",
            )
            assertEquals(
                1L,
                sm.hostNamed(),
                "the completion did not read back the address and the type the host named",
            )
        } finally {
            sm.cleanup()
        }
    }
}
