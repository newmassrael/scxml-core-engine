// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The requester's correlation table and error data, case for case with
// backends/rust/mesh/src/rpc.rs. Each JSON expectation is the literal the Rust
// test pins, so the two cores cannot drift apart without one of them failing.

package com.sce.mesh

import com.sce.generated.rpc_status.RpcStatus
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class RpcTest {
    private val wire = byteArrayOf(
        0x01, 0x92.toByte(), 0x00, 0x00, 0x00, 0x00, 0x70, 0x00,
        0x80.toByte(), 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xab.toByte(),
    )

    @Test
    fun aFailedReplyNamesItsStatusByTheDeclaredName() {
        assertEquals(
            """{"errorName":"invoke","reason":"unavailable","detail":"busy","source":"cloud","invoke_id":"019200000000700080000000000000ab"}""",
            invokeErrorData(RpcStatus.UNAVAILABLE, "busy", "cloud", hex(wire)),
        )
    }

    @Test
    fun aSynthesisedDeadlineHasNoSource() {
        assertEquals(
            """{"errorName":"invoke","reason":"deadlineExceeded","invoke_id":"019200000000700080000000000000ab"}""",
            invokeErrorData(RpcStatus.DEADLINE_EXCEEDED, null, null, hex(wire)),
        )
    }

    @Test
    fun aRequestWithNoRouteIsSrcNotFound() {
        assertEquals(
            """{"errorName":"execution","reason":"INVOKE_SRC_NOT_FOUND","detail":"no binding for '#cloud'"}""",
            srcNotFoundData("no binding for '#cloud'"),
        )
    }

    @Test
    fun anUndeclaredResponderNamesItsSourceAndTheRequest() {
        assertEquals(
            """{"errorName":"communication","reason":"RPC_REPLY_FROM_UNDECLARED_PEER","source":"mallory","invoke_id":"019200000000700080000000000000ab"}""",
            Signal.RpcReplyFromUndeclaredPeer("mallory", hex(wire)).eventData(Binding("hmi", "wss")),
        )
    }

    private fun pending(invokeId: String, token: Long, expiresMs: Long?) =
        Pending(invokeId, token, "cloud", listOf("cloud"), expiresMs)

    /** §mesh-16.7 row 5: a lost link forgets what was sent on it, and only that. */
    @Test
    fun aLostLinkForgetsOnlyTheRequestsSentOnIt() {
        val table = Correlation()
        val other = wire.copyOf().also { it[15] = 0xac.toByte() }
        table.register(wire, pending("ask", 1, 100))
        table.register(other, pending("look", 2, null).copy(target = "hmi"))
        assertEquals(listOf(hex(wire)), table.forgetSentTo("cloud"))
        assertTrue(!table.isWaiting(wire))
        assertTrue(table.isWaiting(other))
        assertTrue(table.expire(1000).isEmpty(), "a forgotten request has no deadline")
    }

    @Test
    fun aRequestWhoseLinkIsLostNamesItAndItsTarget() {
        assertEquals(
            """{"errorName":"communication","reason":"INVOKE_CHILD_LOST","invoke_id":"019200000000700080000000000000ab","target":"hmi"}""",
            Signal.InvokeChildLost(hex(wire), "hmi").eventData(Binding("hmi", "wss")),
        )
    }

    /** §mesh-14.6: only a declared responder retires a request, and a rejected reply leaves it answerable. */
    @Test
    fun onlyADeclaredResponderIsAdmitted() {
        val table = Correlation()
        table.register(wire, pending("ask", 1, null))
        assertEquals(Answer.UNDECLARED, table.check(wire, "mallory"))
        assertEquals(Answer.ADMITTED, table.check(wire, "cloud"))
        assertTrue(table.retire(wire) != null)
        assertEquals(Answer.UNKNOWN, table.check(wire, "cloud"))
    }

    /** A cancel forgets only the start it names: a state entered again starts the same invoke id under a new token. */
    @Test
    fun aCancelForgetsOnlyItsOwnStart() {
        val table = Correlation()
        val second = wire.copyOf().also { it[15] = 0xac.toByte() }
        table.register(wire, pending("ask", 1, null))
        table.register(second, pending("ask", 2, null))
        table.cancel("ask", 1)
        assertEquals(Answer.UNKNOWN, table.check(wire, "cloud"))
        assertEquals(Answer.ADMITTED, table.check(second, "cloud"))
    }

    @Test
    fun aDeadlineExpiresAtItsInstantAndNotBefore() {
        val table = Correlation()
        table.register(wire, pending("ask", 1, 100))
        assertTrue(table.expire(99).isEmpty())
        val expired = table.expire(100)
        assertEquals(1, expired.size)
        assertEquals("ask", expired[0].second.invokeId)
        assertTrue(table.expire(1000).isEmpty())
    }
}
