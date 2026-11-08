// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The causal route through the router (docs/adr/0085, sections 3.2 and 9.2),
// case for case with the causal tests of backends/rust/mesh/src/router.rs.
// Each names the change that turns it red.

package com.sce.mesh

import com.sce.generated.envelope.Envelope
import com.sce.generated.pattern_kind.PatternKind
import com.sce.generated.payload_codec.PayloadCodec
import com.sce.runtime.StateMachineEngine
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlin.test.fail

class CausalRouteTest {
    /**
     * A causal group of three, every copy one machine `list` on its own
     * device; the binding `#people` is the group's route. Unbuffered, so a
     * release is transmitted at once.
     */
    private val people = PeerConfig(
        transport = "wss",
        buffer = null,
        retry = null,
        stampSequence = false,
        delivery = Delivery(dedup = false, ordered = false),
        responders = listOf("people"),
        deadlineMs = null,
        replyEvents = emptyList(),
        causal = CausalGroup("people", 0uL, listOf("phone/list", "laptop/list", "tablet/list")),
    )

    /** A binding that is not causal. */
    private val plain = people.copy(delivery = Delivery(dedup = true, ordered = false), causal = null)

    private fun request(event: String, data: String) =
        StateMachineEngine.HostSendRequest(
            processorType = MESH_PROCESSOR_TYPE,
            eventName = event,
            target = "#people",
            sendId = "",
            eventData = data,
        )

    private fun id(n: Int) = ByteArray(16) { n.toByte() }

    /** The copy of the list whose replica is `[n; 16]`, started. */
    private fun copy(n: Int): Router {
        val router = Router("list", 8u, 50)
        router.addPeer("people", people)
        assertNull(router.startCausal("people", CausalStart.NewIncarnation(Replica.fromBytes(id(n))), 64, 100))
        return router
    }

    private fun done(routed: Routed): List<Effect> = when (routed) {
        is Routed.Done -> routed.effects
        is Routed.Refused -> fail("refused: ${routed.error}")
    }

    /** The single dot an offer was accepted with. */
    private fun accepted(routed: Routed): Dot {
        val effects = done(routed)
        val only = effects.singleOrNull() as? Effect.Accepted ?: fail("expected one accepted change, got $effects")
        assertEquals("people", only.peer)
        return only.dot
    }

    private fun sent(routed: Routed): List<ByteArray> =
        done(routed).map { effect ->
            val transmit = effect as? Effect.Transmit ?: fail("expected only transmissions, got $effect")
            assertEquals("people", transmit.peer)
            transmit.bytes
        }

    private fun events(effects: List<Effect>): List<String> =
        effects.map { (it as? Effect.Deliver)?.event ?: fail("expected only deliveries, got $effects") }

    private fun announceDue(effects: List<Effect>): Boolean = effects.any { it == Effect.AnnounceDue("people") }

    /**
     * Write-ahead. Red if the router transmits a causal change when the layer
     * tags it (a causal send routed to the plain send path): the send then
     * leaves before the journal commit.
     */
    @Test
    fun aCausalSendLeavesOnlyOnceItsJournalRecordIsCommitted() {
        val phone = copy(1)
        val laptop = copy(2)
        val dot = accepted(phone.send(request("edit", """{"n":1}"""), id(9), 0))
        assertEquals(1uL, dot.seq)
        val released = sent(phone.committed("people", dot, 0))
        assertEquals(1, released.size)
        assertEquals(listOf("edit"), events(done(laptop.receive("people", released[0], 0))))
        // The same change again is dropped by its dot, not delivered twice.
        assertTrue(done(laptop.receive("people", released[0], 0)).isEmpty())
    }

    /** A causal peer the host has not started is the host's error, and so is starting a peer that is not causal. */
    @Test
    fun aCausalPeerIsUsedOnlyOnceStarted() {
        val router = Router("list", 8u, 50)
        router.addPeer("people", people)
        router.addPeer("hmi", plain)
        assertEquals(
            Routed.Refused(RouterError.CausalNotStarted("people")),
            router.send(request("edit", ""), id(1), 0),
        )
        assertEquals(
            RouterError.NotCausal("hmi"),
            router.startCausal("hmi", CausalStart.NewIncarnation(Replica.fromBytes(id(1))), 8, 100),
        )
        assertEquals(Routed.Refused(RouterError.NotCausal("hmi")), router.announce("hmi", id(2), 0))
        assertEquals(
            RouterError.UnknownPeer("nobody"),
            router.startCausal("nobody", CausalStart.NewIncarnation(Replica.fromBytes(id(1))), 8, 100),
        )
    }

    /**
     * Repair, on a link that comes back. Red if a peer that becomes ready asks
     * for no vector: the lost change then waits for the period.
     */
    @Test
    fun aLinkThatComesBackSendsAVectorAndRepairsWhatWasLost() {
        val phone = copy(1)
        val laptop = copy(2)
        val dot = accepted(phone.send(request("edit", ""), id(9), 0))
        sent(phone.committed("people", dot, 0))
        val ready = done(laptop.peerReady("people", 10))
        assertTrue(announceDue(ready), "$ready")
        val vector = sent(laptop.announce("people", id(3), 10))
        assertEquals(1, vector.size)
        val answer = sent(phone.receive("people", vector[0], 10))
        assertEquals(1, answer.size)
        assertEquals(listOf("edit"), events(done(laptop.receive("people", answer[0], 10))))
        // A peer that is not causal asks for no vector when it comes back.
        val other = Router("list", 8u, 50)
        other.addPeer("people", plain)
        assertFalse(announceDue(done(other.peerReady("people", 10))))
    }

    /**
     * The period. Red if the tick never asks for a vector, or asks only past
     * the period rather than at it: a member that lost a change and whose
     * links never drop is then never repaired, or later than its period.
     */
    @Test
    fun aCausalPeerIsOwedAVectorOnItsPeriod() {
        val phone = copy(1)
        assertTrue(announceDue(done(phone.tick(0))))
        done(phone.announce("people", id(3), 0))
        assertFalse(announceDue(done(phone.tick(99))))
        assertTrue(announceDue(done(phone.tick(100))))
    }

    /**
     * A period measured across the clock's whole range is due rather than
     * wrapped to a negative age. Red if the age is a plain subtraction.
     */
    @Test
    fun aPeriodAcrossTheWholeClockIsDueNotWrapped() {
        val phone = copy(1)
        done(phone.announce("people", id(3), Long.MIN_VALUE))
        assertTrue(announceDue(done(phone.tick(Long.MAX_VALUE))))
    }

    /**
     * The view. Red if the epoch check is removed: a change tagged with another
     * epoch is then delivered rather than raised as row 18.
     */
    @Test
    fun aChangeOfAnotherViewIsRaisedAndNotDelivered() {
        val laptop = copy(2)
        val envelope = Envelope(
            id = id(5),
            source = "list",
            event_type = "edit",
            pattern = PatternKind.FIRE_FORGET,
            datacontenttype = PayloadCodec.NONE,
        )
        val tag = CausalTag(1uL, listOf(Replica.fromBytes(id(1)).entry(1uL)))
        val bytes = stamp(envelope, tag).encodeToByteArray() ?: fail("encodes")
        assertEquals(
            listOf(Effect.Raise("people", Signal.CausalViewMismatch("list", 1uL, "01010101-0101-0101-0101-010101010101"))),
            done(laptop.receive("people", bytes, 0)),
        )
    }

    /** An envelope on a causal route without its tag cannot be placed: it is corrupt, not delivered. */
    @Test
    fun anUntaggedEnvelopeOnACausalRouteIsCorrupt() {
        val laptop = copy(2)
        val bytes = Envelope(
            id = id(5),
            source = "list",
            event_type = "edit",
            pattern = PatternKind.FIRE_FORGET,
            datacontenttype = PayloadCodec.NONE,
        ).encodeToByteArray() ?: fail("encodes")
        assertEquals(
            listOf(Effect.Raise("people", Signal.EnvelopeCorrupt("list", "cbor"))),
            done(laptop.receive("people", bytes, 0)),
        )
        assertEquals(
            listOf(Effect.Raise("people", Signal.EnvelopeCorrupt(null, "cbor"))),
            done(laptop.receive("people", byteArrayOf(0xFF.toByte()), 0)),
        )
    }

    /** A small deterministic generator for the world below. */
    private class Rng(private var state: ULong) {
        fun step(): ULong {
            state = state xor (state shl 13)
            state = state xor (state shr 7)
            state = state xor (state shl 17)
            return state
        }

        fun below(n: Int): Int = (step() % n.toULong()).toInt()

        fun chance(percent: Int): Boolean = step() % 100uL < percent.toULong()
    }

    /**
     * Three copies of the list, the bytes in flight to each, and what each has
     * taken in: per author, how many of its changes.
     */
    private inner class World(seed: ULong) {
        val copies = (1..3).map { copy(it) }
        val wire = mutableListOf<Pair<Int, ByteArray>>()
        val seen = Array(3) { IntArray(3) }
        val rng = Rng(seed)
        var now = 0L
        var lossy = true

        /** An envelope id drawn from the world's generator. */
        fun nextId(): ByteArray = ByteArray(16) { rng.step().toByte() }

        /** Act on what copy [at] was told to do, as its host would. */
        fun act(at: Int, routed: Routed) {
            for (effect in done(routed)) {
                when (effect) {
                    is Effect.Transmit -> for (target in 0 until 3) {
                        if (target == at || (lossy && rng.chance(30))) continue
                        if (lossy && rng.chance(10)) wire += Pair(target, effect.bytes)
                        wire += Pair(target, effect.bytes)
                    }
                    is Effect.Accepted -> act(at, copies[at].committed(effect.peer, effect.dot, now))
                    is Effect.AnnounceDue -> act(at, copies[at].announce(effect.peer, nextId(), now))
                    is Effect.Deliver -> take(at, effect.data)
                    else -> fail("copy $at: unexpected $effect")
                }
            }
        }

        /**
         * Copy [at] takes in the change [data] names: `<author> <n> <seen of
         * author 0> <of 1> <of 2>`, what its author had taken in when it made
         * it. It must be the author's next change, and everything its author
         * had taken in must be taken in here already.
         */
        fun take(at: Int, data: String) {
            val fields = data.split(' ').map { it.toInt() }
            val author = fields[0]
            assertEquals(seen[at][author] + 1, fields[1], "copy $at takes in $data once and skips no gap")
            for (other in 0 until 3) {
                if (other != author) {
                    assertTrue(seen[at][other] >= fields[2 + other], "copy $at takes in $data before a change it depends on")
                }
            }
            seen[at][author] = fields[1]
        }

        /** Copy [at] makes its next change. */
        fun make(at: Int) {
            val n = seen[at][at] + 1
            val data = "$at $n ${seen[at][0]} ${seen[at][1]} ${seen[at][2]}"
            seen[at][at] = n
            act(at, copies[at].send(request("edit", data), nextId(), now))
        }

        fun deliverOne() {
            val (target, bytes) = wire.removeAt(rng.below(wire.size))
            act(target, copies[target].receive("people", bytes, now))
        }
    }

    /**
     * The world. Three copies on links that lose, duplicate and reorder take in
     * every change exactly once, in an order that is a linear extension of what
     * each change's author had taken in, with no gap skipped, and their logs
     * drop to the stable cut once the links are quiet. Red if the hold branch
     * delivers, or if repair is removed (a lost change then never arrives).
     */
    @Test
    fun threeCopiesThroughTheirRoutersTakeInEveryChangeOnceInCausalOrder() {
        val changesEach = 15
        for (seed in listOf(0x0BAD5EEDuL, 0x0CA5CADEuL, 0x00C0FFEEuL)) {
            val world = World(seed)
            while ((0 until 3).any { world.seen[it][it] < changesEach } || world.wire.isNotEmpty()) {
                world.now += 1
                when (world.rng.below(10)) {
                    0, 1, 2 -> {
                        val at = world.rng.below(3)
                        if (world.seen[at][at] < changesEach) world.make(at)
                    }
                    3 -> {
                        val at = world.rng.below(3)
                        world.act(at, world.copies[at].tick(world.now))
                    }
                    else -> if (world.wire.isNotEmpty()) world.deliverOne()
                }
            }
            world.lossy = false
            repeat(20) {
                world.now += 1
                for (at in 0 until 3) {
                    world.act(at, world.copies[at].announce("people", world.nextId(), world.now))
                }
                while (world.wire.isNotEmpty()) world.deliverOne()
            }
            val label = "seed ${seed.toString(16)}"
            for (at in 0 until 3) {
                assertEquals(List(3) { changesEach }, world.seen[at].toList(), "$label: copy $at takes in every change")
                val layer = world.copies[at].causalLayer("people") ?: fail("$label: copy $at has its layer")
                assertEquals(0, layer.heldLen, "$label: nothing held")
                assertEquals(0, layer.logLen, "$label: the log is at the cut")
            }
        }
    }
}
