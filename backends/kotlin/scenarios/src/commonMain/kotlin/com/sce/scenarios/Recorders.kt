// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What a machine asked of its hosts and sent its peers, as a scenario writes it.
//
// A generated machine with `<sce:action>`s takes a host that performs them, and
// the generator writes one that only records them (`Recording<Machine>Actions`).
// A driver turns what that host recorded into the entries of a scenario's
// `host_calls` with [hostCall], and hands back only what is new since it was
// last asked, which is how a step is judged on the calls of that step alone.
//
// A child session is given a host of its own by its parent's host, and the
// child owns it: nothing outside the machine can read what the child asked of
// it. [ChildHosts] is the book every such host is written into, and the one
// thing a scenario's `child_hosts` is read from. [RecordedSends] is the Mesh
// router a driver registers, which only writes down what the machine sent.

package com.sce.scenarios

import com.sce.runtime.StateMachineEngine
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

/**
 * One call of a host as a scenario's `host_calls` writes it: the name the
 * document gives the `<sce:action>` and its arguments in the order the document
 * gives them, `{"action": "showAttempts", "args": [0, false]}`.
 */
fun hostCall(action: String, vararg args: JsonElement): JsonElement =
    buildJsonObject {
        put("action", action)
        put("args", JsonArray(args.toList()))
    }

/**
 * The child hosts a machine's host was asked for, in the order asked, and what
 * each child asked of its host. A child owns the host it was given, so the host
 * the parent answers is written here as it is made ([open]), with the function
 * that reads what it recorded as the scenario writes it and forgets it. Its
 * [take] is what a driver hands [replay] as `childHosts`.
 */
class ChildHosts {
    private class Child(
        val invoke: String,
        val start: Int,
        val candidate: String?,
        val take: () -> List<JsonElement>,
    ) {
        var announced = false
    }

    private val children = mutableListOf<Child>()

    /**
     * The machine's host was asked for the host of a child its [invoke]
     * started, and answers [host]: [candidate] is the stem of the document when
     * the invoke names one of several, and [take] reads what the host recorded
     * since it was last read, as `host_calls` writes it ([hostCall]), and
     * forgets it. Returns [host], so the call is the answer a generated
     * recording host's factory gives.
     */
    fun <H> open(invoke: String, candidate: String?, host: H, take: (H) -> List<JsonElement>): H {
        val start = 1 + children.count { it.invoke == invoke }
        children += Child(invoke, start, candidate) { take(host) }
        return host
    }

    /**
     * The children asked for or called since this was last called, oldest asked
     * first, each as a scenario's `child_hosts` writes it: the invoke, the start
     * of that invoke, the candidate's document stem where the invoke names one,
     * and the calls made in this step alone. A child asked for earlier that
     * called nothing since has no entry.
     */
    fun take(): List<JsonElement> =
        children.mapNotNull { child ->
            val calls = child.take()
            if (child.announced && calls.isEmpty()) {
                return@mapNotNull null
            }
            child.announced = true
            buildJsonObject {
                put("invoke", child.invoke)
                put("start", child.start)
                child.candidate?.let { put("candidate", it) }
                put("host_calls", JsonArray(calls))
            }
        }
}

/**
 * What a machine sent its Mesh peers (a `<send>` whose target is `#name`) as
 * the router a host registers is handed it. Register [router] with the machine's
 * `registerMeshRouter` before it starts, so that what its start sends is
 * recorded too; with no router registered such a send is `error.execution` and
 * reaches nobody. The peer is not there: what a scenario holds a step to is that
 * the machine asked for the send. Its [take] is what a driver hands [replay] as
 * `sent`.
 */
class RecordedSends {
    private val requests = mutableListOf<StateMachineEngine.HostSendRequest>()

    /** The router to register: it writes the send down and answers nothing. */
    val router: (StateMachineEngine.HostSendRequest) -> List<StateMachineEngine.HostSendResponse> = { request ->
        requests += request
        emptyList()
    }

    /**
     * The sends made since this was last called, oldest first, each as a
     * scenario's `sent` writes it: the target as the document writes it, the
     * event's name, and the event's data as the JSON value its text spells,
     * left out when the send carried none.
     */
    fun take(): List<JsonElement> {
        val fresh =
            requests.map { request ->
                buildJsonObject {
                    put("target", request.target)
                    put("event", request.eventName)
                    if (request.eventData.isNotEmpty()) {
                        put("data", Json.parseToJsonElement(request.eventData))
                    }
                }
            }
        requests.clear()
        return fresh
    }
}
