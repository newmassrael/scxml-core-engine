// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2.4: delayed sends that come due together are delivered in the
// order they were sent — Kotlin coroutine mode (`start(scope)`).
//
// The synchronous engine keeps its waiting sends in one time-ordered list with a
// sequence number, so two sends of the same delay come out in send order by
// construction. Coroutine mode used to give each plain delayed send a coroutine
// of its own, which wakes on whatever thread the pool gives it: the order sends
// of one delay arrived in was the order the pool woke them in. The jobs also lived
// in a map the loop and the woken coroutines both wrote, and a woken job removed
// its entry by key, which removes the entry of a send that replaced it.
//
// One list, performed by the loop's own thread, is what every other waiting act
// in this mode already used (a host-served send, a parent send, an invoke's
// deadline), and is what a plain delayed send uses now.
//
// This runs on the real clock, because coroutine mode is the mode a host does not
// drive by hand; the delay is short and the assertion is on order, which no load
// on the machine can make come out right by accident for forty sends.

package com.sce.integration

import com.sce.runtime.EnabledTransition
import com.sce.runtime.Event
import com.sce.runtime.HistoryId
import com.sce.runtime.State
import com.sce.runtime.StateMachineEngine
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import java.util.concurrent.CopyOnWriteArrayList

private object OrderedSendState : State

private sealed interface OrderedSendEvent : Event {
    data object Kick : OrderedSendEvent
    data object CancelAll : OrderedSendEvent
    data object Replace : OrderedSendEvent
    data object Chain : OrderedSendEvent
    data class Tick(val n: Int) : OrderedSendEvent
}

/**
 * The smallest machine that sends from inside a macrostep, as a generated one
 * does: `Kick` arms [SENDS] sends of one delay, in order, `CancelAll` cancels
 * the even ones, `Replace` arms two sends under one id, and `Chain` arms two
 * sends of one delay of which the earlier one's transition cancels the later.
 */
private class OrderedSendMachine : StateMachineEngine<OrderedSendState, OrderedSendEvent>() {
    override val initialState: OrderedSendState = OrderedSendState

    /** What the machine was asked to take, in order. Read from the test's thread. */
    val taken = CopyOnWriteArrayList<OrderedSendEvent>()

    private fun transition(index: Int) =
        EnabledTransition<OrderedSendState, HistoryId>(OrderedSendState, emptyList(), index, hasActions = true, isInternal = false)

    override fun firstEnabledTransition(state: OrderedSendState, event: OrderedSendEvent?): EnabledTransition<OrderedSendState, HistoryId>? =
        when (event) {
            OrderedSendEvent.Kick -> transition(KICK)
            OrderedSendEvent.CancelAll -> transition(CANCEL_ALL)
            OrderedSendEvent.Replace -> transition(REPLACE)
            OrderedSendEvent.Chain -> transition(CHAIN)
            null -> null
            is OrderedSendEvent.Tick -> {
                taken.add(event)
                if (event.n == EARLIER_OF_CHAIN) transition(CANCEL_NEXT) else null
            }
        }

    override fun onEntry(state: OrderedSendState, isDefaultEntry: Boolean) {}

    override fun onExit(state: OrderedSendState) {}

    override fun executeTransitionContent(source: OrderedSendState, transitionIndex: Int) {
        when (transitionIndex) {
            KICK -> for (n in 0 until SENDS) scheduleSend("s$n", DELAY_MS, OrderedSendEvent.Tick(n))
            CANCEL_ALL -> for (n in 0 until SENDS step 2) cancelSend("s$n")
            REPLACE -> {
                scheduleSend("r", DELAY_MS, OrderedSendEvent.Tick(FIRST_OF_PAIR))
                scheduleSend("r", DELAY_MS, OrderedSendEvent.Tick(SECOND_OF_PAIR))
            }
            CHAIN -> {
                scheduleSend("c0", DELAY_MS, OrderedSendEvent.Tick(EARLIER_OF_CHAIN))
                scheduleSend("c1", DELAY_MS, OrderedSendEvent.Tick(LATER_OF_CHAIN))
            }
            CANCEL_NEXT -> cancelSend("c1")
        }
    }

    override fun resolveState(stateId: String): OrderedSendState? = OrderedSendState.takeIf { stateId == "only" }

    override fun stateIdOf(state: OrderedSendState): String = "only"

    override fun resolveEventByName(name: String): OrderedSendEvent? = null

    override fun eventNameOf(event: OrderedSendEvent): String? = when (event) {
        OrderedSendEvent.Kick -> "kick"
        OrderedSendEvent.CancelAll -> "cancel_all"
        OrderedSendEvent.Replace -> "replace"
        OrderedSendEvent.Chain -> "chain"
        is OrderedSendEvent.Tick -> "tick"
    }

    companion object {
        const val SENDS = 40
        const val DELAY_MS = 150L
        const val FIRST_OF_PAIR = 100
        const val SECOND_OF_PAIR = 101
        const val EARLIER_OF_CHAIN = 200
        const val LATER_OF_CHAIN = 201
        private const val KICK = 0
        private const val CANCEL_ALL = 1
        private const val REPLACE = 2
        private const val CHAIN = 3
        private const val CANCEL_NEXT = 4
    }
}

@DisplayName("Sends of one delay arrive in the order they were sent (§6.2.4)")
class ASendsOfOneDelayArriveInTheOrderTheyWereSentTest {

    private fun <T> running(body: (OrderedSendMachine) -> T): T {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        val sm = OrderedSendMachine()
        try {
            sm.start(scope)
            return body(sm)
        } finally {
            sm.stop()
            scope.cancel()
            sm.cleanup()
        }
    }

    private fun awaitTaken(sm: OrderedSendMachine, count: Int) = runBlocking {
        withTimeout(10_000) {
            while (sm.taken.size < count) delay(10)
        }
    }

    @Test
    fun sendsOfOneDelayComeInTheOrderTheyWereSent() {
        running { sm ->
            sm.send(OrderedSendEvent.Kick)
            awaitTaken(sm, OrderedSendMachine.SENDS)

            assertEquals(
                (0 until OrderedSendMachine.SENDS).map { OrderedSendEvent.Tick(it) },
                sm.taken.toList(),
                "forty sends armed in one macrostep for one delay, delivered in the order they were armed",
            )
        }
    }

    @Test
    fun aCancelledSendIsNeverDeliveredAndTheOthersKeepTheirOrder() {
        running { sm ->
            sm.send(OrderedSendEvent.Kick)
            sm.send(OrderedSendEvent.CancelAll)
            awaitTaken(sm, OrderedSendMachine.SENDS / 2)
            // Past the delay by a clear margin: a cancelled send that was going
            // to arrive anyway has arrived by now.
            runBlocking { delay(OrderedSendMachine.DELAY_MS * 2) }

            assertEquals(
                (1 until OrderedSendMachine.SENDS step 2).map { OrderedSendEvent.Tick(it) },
                sm.taken.toList(),
                "the even sends were cancelled while they waited; the odd ones arrived in send order",
            )
        }
    }

    @Test
    fun aSendReplacedUnderTheSameIdIsDeliveredOnlyOnce() {
        running { sm ->
            sm.send(OrderedSendEvent.Replace)
            awaitTaken(sm, 1)
            runBlocking { delay(OrderedSendMachine.DELAY_MS * 2) }

            assertEquals(
                listOf<OrderedSendEvent>(OrderedSendEvent.Tick(OrderedSendMachine.SECOND_OF_PAIR)),
                sm.taken.toList(),
                "a second send under an id still waiting replaces it: only the later one arrives",
            )
        }
    }

    /**
     * Two sends come due together. The macrostep the earlier one starts cancels
     * the later one, and a cancel has to reach it: sends are taken one at a
     * time, with that macrostep between them, and are not all queued at once.
     */
    @Test
    fun aSendCancelledByTheMacrostepOfAnEarlierOneNeverArrives() {
        running { sm ->
            sm.send(OrderedSendEvent.Chain)
            awaitTaken(sm, 1)
            runBlocking { delay(OrderedSendMachine.DELAY_MS) }

            assertEquals(
                listOf<OrderedSendEvent>(OrderedSendEvent.Tick(OrderedSendMachine.EARLIER_OF_CHAIN)),
                sm.taken.toList(),
                "the later send was cancelled by the earlier one's transition, before it was taken",
            )
        }
    }
}
