// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A generated machine as an actor of the world simulation (backends/rust/world).
//
// The world owns time, the network and every random choice; this process owns one
// machine and nothing else. It answers a line of JSON with a line of JSON, one request
// at a time, and does nothing between requests: no thread, no timer, no read of the
// wall clock. The machine runs on a `ManualClock` that only an `advance` request moves,
// so what it sends and holds after a sequence of requests is a function of that
// sequence, the same on every run and every host.
//
// The machine is driven as the scenarios every backend replays drive it
// (docs/SCE_ACCEPTED_SUBSET.md §2.15, "A scenario of a static machine"): its Mesh router
// is one that writes the send down and delivers it to no one, the world carries it to the
// peer; an event is given by name with its payload; and after each step the machine is
// given five rounds of `tick`, enough for a child to take an event its parent forwarded
// in one round and for its end to reach the parent in the next, since the engine reports
// no point at which it has settled.
//
// The protocol is documented where it is defined for the other side, in the module
// documentation of `sce_rust_world::external`; this file implements it.

package com.sce.world

import com.sce.runtime.EventMetadata
import com.sce.runtime.ManualClock
import com.sce.runtime.SavedState
import com.sce.runtime.StateMachineEngine
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonArray
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.long
import kotlinx.serialization.json.put

/** The version of the line protocol this server speaks; the world refuses another. */
const val PROTOCOL_VERSION = 1

/**
 * How many rounds of `tick` a machine is given after each step before it is read. The
 * same number every driver of the scenarios runs (`SETTLE_TICKS` in `StaticScenarioTest`).
 */
private const val SETTLE_TICKS = 5

/** A machine a world can host, as the step server needs it. */
interface HostedMachine {
    val engine: StateMachineEngine<*, *>

    /** The machine's saved state: its variables and where it stands. */
    fun save(): SavedState

    /** Stand the machine where [saved] left one, in place of `initialize`. */
    fun restore(saved: SavedState)
}

/**
 * One generated machine, from the three things its class has and no shared interface
 * names: the constructor, and the `save` and `restore` the generator writes for the
 * document.
 */
fun <M : StateMachineEngine<*, *>> hosting(
    create: () -> M,
    save: (M) -> SavedState,
    restore: (M, SavedState) -> Unit,
): () -> HostedMachine = {
    val machine = create()
    object : HostedMachine {
        override val engine: StateMachineEngine<*, *> = machine

        override fun save(): SavedState = save(machine)

        override fun restore(saved: SavedState) = restore(machine, saved)
    }
}

/** A request this server cannot honour, answered as `{"ok":false}` and not as a crash. */
class ProtocolError(message: String) : Exception(message)

/**
 * The server for the machine called [machineName], built by [create].
 *
 * [handle] is the whole protocol: it takes the text of one request and returns the text
 * of its reply. [stopped] says a `shutdown` was answered, after which nothing is read.
 */
class StepServer(
    private val machineName: String,
    private val create: () -> HostedMachine,
) {
    private var hosted: HostedMachine? = null
    private val sends = mutableListOf<StateMachineEngine.HostSendRequest>()

    var stopped: Boolean = false
        private set

    fun handle(line: String): String =
        try {
            val request = Json.parseToJsonElement(line).jsonObject
            reply(request).toString()
        } catch (failure: ProtocolError) {
            refusal(failure.message ?: "refused")
        } catch (failure: Exception) {
            // A machine that throws is a defect for the world to report with the stack
            // that says where, not a reason for this process to die silently.
            refusal(failure.stackTraceToString())
        }

    /** Release the machine, if one is running. */
    fun close() {
        hosted?.engine?.cleanup()
        hosted = null
    }

    private fun refusal(message: String): String =
        buildJsonObject {
            put("ok", false)
            put("error", message)
        }.toString()

    private fun reply(request: JsonObject): JsonObject =
        when (val op = text(request, "op")) {
            "hello" ->
                buildJsonObject {
                    put("ok", true)
                    put("protocol", PROTOCOL_VERSION)
                    put("machine", machineName)
                }
            "start" -> {
                begin(number(request, "at_ms"), restoring = null)
                step()
            }
            "raise" -> {
                val machine = running()
                machine.engine.sendEventByName(
                    text(request, "event"),
                    EventMetadata(
                        data = text(request, "data"),
                        sendId = text(request, "send_id"),
                        origin = text(request, "origin"),
                        originType = text(request, "origin_type"),
                        invokeId = text(request, "invoke_id"),
                    ),
                )
                settle(machine)
                step()
            }
            "advance" -> {
                val machine = running()
                machine.engine.advanceTimeMs(number(request, "ms"))
                settle(machine)
                step()
            }
            "saved" ->
                buildJsonObject {
                    put("ok", true)
                    put("saved", running().save().toJson())
                }
            "restore" -> {
                begin(number(request, "at_ms"), restoring = SavedState.fromJson(text(request, "saved")))
                step()
            }
            "shutdown" -> {
                close()
                stopped = true
                buildJsonObject { put("ok", true) }
            }
            else -> throw ProtocolError("unknown op `$op`")
        }

    /**
     * Replace the machine with a new one whose clock reads [atMs]: started, or, when
     * [restoring] is given, stood where that saved state left one. The router is
     * registered before either, so what a start sends is recorded too.
     */
    private fun begin(atMs: Long, restoring: SavedState?) {
        close()
        sends.clear()
        val machine = create()
        val engine = machine.engine
        engine.clock = ManualClock(atMs)
        engine.registerMeshRouter { request ->
            sends += request
            emptyList()
        }
        hosted = machine
        if (restoring == null) {
            engine.initialize()
        } else {
            machine.restore(restoring)
        }
        settle(machine)
    }

    private fun running(): HostedMachine = hosted ?: throw ProtocolError("no machine is running: `start` or `restore` first")

    private fun settle(machine: HostedMachine) {
        repeat(SETTLE_TICKS) { machine.engine.tick() }
    }

    /** What the machine sent since it was last asked, and when it next has something due. */
    private fun step(): JsonObject {
        val machine = running()
        val taken = sends.toList()
        sends.clear()
        return buildJsonObject {
            put("ok", true)
            put(
                "sent",
                buildJsonArray {
                    taken.forEach { add(describe(it)) }
                },
            )
            val due = machine.engine.timeUntilNextScheduledMs()
            put("next_due_in_ms", due?.let { JsonPrimitive(it) } ?: JsonNull)
        }
    }

    /** A send as the world rebuilds it: every field the engine handed the router. */
    private fun describe(request: StateMachineEngine.HostSendRequest): JsonObject =
        buildJsonObject {
            put("processor_type", request.processorType)
            put("event", request.eventName)
            put("target", request.target)
            put("content", request.content)
            put(
                "params",
                buildJsonObject {
                    request.params.forEach { (name, values) ->
                        put(name, JsonArray(values.map { JsonPrimitive(it) }))
                    }
                },
            )
            put("send_id", request.sendId)
            put("data", request.eventData)
            put("invoke_id", request.invokeId)
        }

    private fun field(request: JsonObject, name: String): JsonElement =
        request[name] ?: throw ProtocolError("the request has no `$name`")

    private fun text(request: JsonObject, name: String): String {
        val value = field(request, name).jsonPrimitive
        if (!value.isString) throw ProtocolError("`$name` is not a string")
        return value.content
    }

    private fun number(request: JsonObject, name: String): Long =
        try {
            field(request, name).jsonPrimitive.long
        } catch (failure: NumberFormatException) {
            throw ProtocolError("`$name` is not a whole number: ${failure.message}")
        }
}
