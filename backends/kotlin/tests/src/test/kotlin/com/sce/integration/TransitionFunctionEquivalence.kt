// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A generated machine held to the transition function generated beside it
// (docs/SCE_TRANSITION_FUNCTION.md).
//
// The function is computed at build time by the selection rule the engine
// follows at run time, so the two are one fact only if every pair agrees, and
// this is what asks: a fresh machine per pair, driven to the state by the
// shortest event path the function itself gives, then sent the event. What the
// engine reaches, and whether it took the event or discarded it, must be what
// the function says. The drivers of the integration stems that declare a
// function share it, so the two stems ask the same question the same way.

package com.sce.integration

import org.junit.jupiter.api.Assertions.assertEquals

/**
 * A generated transition function, read through its accessors: the package is
 * the generator's, so the drive is handed what it needs of it by name.
 */
class TransitionFunctionView<S : Any, E : Any>(
    val states: List<S>,
    val events: List<E>,
    val initial: S,
    val transition: (S, E) -> S?,
    val isFinal: (S) -> Boolean,
    val configuration: (S) -> List<String>,
    val stateName: (S) -> String,
    val eventName: (E) -> String,
)

/**
 * What the drive asks of the engine it holds to a function: a fresh machine at
 * its initial configuration, an event delivered by name and run to completion,
 * the ids of the active states, the final the run ended in, and how many
 * events the engine has discarded.
 */
class EngineProbe(
    val start: () -> Unit,
    val send: (String) -> Unit,
    val active: () -> Set<String>,
    val terminal: () -> String?,
    val discarded: () -> Int,
)

/**
 * The shortest sequence of events that takes [function] from its initial state
 * to [target], read from the function alone; `null` when no sequence does.
 */
fun <S : Any, E : Any> shortestPath(function: TransitionFunctionView<S, E>, target: S): List<E>? {
    val frontier = mutableListOf(Pair(function.initial, emptyList<E>()))
    val seen = mutableSetOf(function.initial)
    var next = 0
    while (next < frontier.size) {
        val (state, path) = frontier[next]
        next++
        if (state == target) return path
        for (event in function.events) {
            val to = function.transition(state, event)
            if (to != null && seen.add(to)) {
                frontier.add(Pair(to, path + event))
            }
        }
    }
    return null
}

/**
 * Drive [engine] to every state of [function] by the shortest path the function
 * gives, send it every event there, and assert that the configuration it
 * reaches, and whether it took the event or discarded it, is what the function
 * says. The configuration is compared as a set of ids: a state's ancestors are
 * fixed by the document, so the set says what the ordered list says. Returns
 * how many pairs it checked, which a caller holds to the number of states
 * times the number of events.
 */
fun <S : Any, E : Any> expectEveryPairAgrees(function: TransitionFunctionView<S, E>, engine: EngineProbe): Int {
    engine.start()
    assertEquals(
        function.configuration(function.initial).toSet(),
        engine.active(),
        "the machine does not start where the function does",
    )
    var pairs = 0
    for (state in function.states) {
        val name = function.stateName(state)
        val path = checkNotNull(shortestPath(function, state)) { "$name is not reached from the initial state" }
        for (event in function.events) {
            val pair = "($name, ${function.eventName(event)})"
            engine.start()
            for (taken in path) engine.send(function.eventName(taken))
            pairs++
            if (function.isFinal(state)) {
                assertEquals(name, engine.terminal(), "$pair: the path does not end the machine there")
                continue
            }
            assertEquals(
                function.configuration(state).toSet(),
                engine.active(),
                "$pair: the path leaves the machine elsewhere",
            )
            val discarded = engine.discarded()
            engine.send(function.eventName(event))
            val to = function.transition(state, event)
            if (to == null) {
                assertEquals(discarded + 1, engine.discarded(), "$pair: the function refuses it and the machine took it")
                assertEquals(
                    function.configuration(state).toSet(),
                    engine.active(),
                    "$pair: the machine moved on an event it discarded",
                )
            } else {
                assertEquals(discarded, engine.discarded(), "$pair: the function takes it and the machine discarded it")
                if (function.isFinal(to)) {
                    assertEquals(
                        function.stateName(to),
                        engine.terminal(),
                        "$pair: the machine did not end where the function says",
                    )
                } else {
                    assertEquals(
                        function.configuration(to).toSet(),
                        engine.active(),
                        "$pair: the machine is not where the function says",
                    )
                }
            }
        }
    }
    return pairs
}
