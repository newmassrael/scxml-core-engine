// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Replay of an engine-neutral scenario against a generated Kotlin machine
// (`datamodel="sce-static"`, docs/SCE_ACCEPTED_SUBSET.md §2.15).
//
// A scenario (`sce-build/tests/fixtures/static_datamodel/scenarios/*.json` in
// this repository, or any file of the same form) is data: a list of steps, each
// an external event with its payload and what the machine must hold after it
// runs to quiescence: its current state, the set of its active states
// (`configuration`), any of its variables, what it asked of its host
// (`host_calls`), what each child session was given as its host and asked of it
// (`child_hosts`) and what it sent its Mesh peers (`sent`). What the machine
// holds is read from its saved state, the text every backend saves byte for
// byte, so one scenario judges every backend by the same answer and needs no
// per-type glue.

package com.sce.scenarios

import com.sce.runtime.SavedState
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

/**
 * How many rounds a host gives a machine that needs `tick` after an event before
 * it reads the machine back. The engine reports no point at which it has settled,
 * so a host runs rounds: a child session takes an event the parent forwarded in
 * one round and its end reaches the parent in the next, which is two for each
 * level of child. Five is what every backend's suite gives a machine, and a round
 * that finds nothing to do changes nothing.
 */
const val SETTLE_TICKS: Int = 5

/** The scenario [text] holds, read as the JSON object a scenario is. */
fun parseScenario(text: String): JsonObject = Json.parseToJsonElement(text).jsonObject

/**
 * Replay [scenario] against a machine driven by [send] and [tick], read back with
 * [save] after every step, and asked whether it has [ended].
 *
 * The driver is a set of functions rather than a machine, so the replay holds
 * any generated machine the same way and needs no type of it:
 *
 * - [send] raises an external event by its name with its data as JSON text (the
 *   empty text when the step gives none); [tick] then runs the machine to
 *   quiescence. [tick] is called once before the first step, too: a machine that
 *   starts a child session in its first state has that child running, and maybe
 *   ended, before any event arrives. A machine with child sessions needs
 *   [SETTLE_TICKS] rounds where a plain one needs one.
 * - [save] is the machine's saved state, read after every step that does not
 *   end it. [ended] says whether it ended in a top-level `<final>`, and
 *   [donedata] is then the JSON the final's done event carries.
 * - [advance] moves the machine's manual clock on, for a step that names an
 *   `advance_ms`; a machine that runs on no manual clock gives none.
 * - [hostCalls], [childHosts] and [sent] read what the machine asked of its own
 *   host, of the hosts its children were given, and what it sent its Mesh peers,
 *   since each was last read, as a scenario writes them ([hostCall],
 *   [ChildHosts.take], [RecordedSends.take]). Each is read on every step, so a
 *   step that states nothing does not hand what it read to the next.
 * - [children] are the routers the driver registered for the machine's child
 *   sessions (SCE_MESH.md, Child Sessions in Mesh): a step's `invoke` hands its
 *   event to the running child of that path through them, as a peer's event,
 *   and the machine applies it when it next drives the child.
 *
 * A step that states what the driver does not record (`host_calls` with no
 * [hostCalls], an `advance_ms` with no [advance], an `invoke` with no
 * [children]) fails: a step that judged nothing would pass. So does an `invoke`
 * that names no running child.
 *
 * An event name no event of the machine matches is refused by the Rust and Go
 * twins, which ask the machine's own lookup unless the step says
 * `"dropped": true`; a Kotlin machine does not make that lookup public, so a step
 * whose event was dropped fails on the state it expects, or, for a step that
 * expects the drop, on the variables it says did not move.
 *
 * @throws ScenarioMismatch on the first step the machine does not hold, naming
 *   the step, its note and what differed.
 */
fun replay(
    scenario: JsonObject,
    send: (String, String) -> Unit,
    tick: () -> Unit,
    save: () -> SavedState,
    ended: () -> Boolean,
    donedata: () -> String = { "" },
    advance: ((Long) -> Unit)? = null,
    hostCalls: (() -> List<JsonElement>)? = null,
    childHosts: (() -> List<JsonElement>)? = null,
    sent: (() -> List<JsonElement>)? = null,
    children: ChildRouters? = null,
) {
    val steps = scenario.getValue("steps").jsonArray
    if (steps.isEmpty()) {
        throw ScenarioMismatch(null, "", "a scenario with no steps judges nothing")
    }
    tick()
    steps.forEachIndexed { n, element ->
        val step = element.jsonObject
        val note = step["note"]?.jsonPrimitive?.content ?: ""
        // A step that moves the machine's time on, for a scenario of a delayed
        // send: the machine runs on a manual clock, so a wait is the one the
        // step names and not the one the caller happened to take.
        step["advance_ms"]?.jsonPrimitive?.content?.let { ms ->
            if (advance == null) {
                throw ScenarioMismatch(
                    n,
                    note,
                    "this driver advances no manual clock, so it cannot hold a step to a wait",
                )
            }
            advance(ms.toLong())
        }
        step["event"]?.jsonPrimitive?.content?.let { event ->
            val data = step["data"]?.toString() ?: ""
            val path = step["invoke"]?.jsonPrimitive?.content
            if (path == null) {
                send(event, data)
            } else if (children == null) {
                throw ScenarioMismatch(
                    n,
                    note,
                    "this driver gives children no routers, so it cannot send `$event` to `$path`",
                )
            } else if (!children.deliver(path, event, data)) {
                throw ScenarioMismatch(n, note, "no child of `$path` is running to be sent `$event`")
            }
            tick()
        }
        val expect = step.getValue("expect").jsonObject
        // What the machine asked of its host in this step. Read on every step
        // so that a step which states nothing does not hand its calls to the
        // next, and a step that states some needs a driver that records them.
        holdRecorded(n, note, expect["host_calls"], hostCalls?.invoke(), "host calls", "what the machine asked of its host")
        // The child hosts the machine's host was asked for in this step, and what
        // each child asked of its host, read and held the same way.
        holdRecorded(
            n,
            note,
            expect["child_hosts"],
            childHosts?.invoke(),
            "child hosts",
            "the child hosts the machine's host was asked for, and what their children asked of them",
        )
        // What the machine sent its Mesh peers in this step, read and held to
        // the step the same way.
        holdRecorded(n, note, expect["sent"], sent?.invoke(), "sends to Mesh peers", "what the machine sent its Mesh peers")
        // A machine that ended in a top-level <final> has no saved state to
        // read (the save refuses one), so what a scenario can say of it is
        // that it ended, and that is the whole of the step.
        if (expect["ended"]?.jsonPrimitive?.content == "true") {
            if (expect["state"] != null || expect["variables"] != null) {
                throw ScenarioMismatch(n, note, "an ended machine has no state or variables to read")
            }
            if (!ended()) {
                throw ScenarioMismatch(
                    n,
                    note,
                    "whether the machine ended in a top-level <final>",
                    expected = JsonPrimitive(true),
                    actual = JsonPrimitive(false),
                )
            }
            // The data its <donedata> left for the invoking parent, as the
            // JSON the done event carries; compared as a value, so the order
            // the pairs were written in is not part of the answer.
            expect["donedata"]?.let { want ->
                val got = Json.parseToJsonElement(donedata())
                if (got != want) {
                    throw ScenarioMismatch(n, note, "the data the final's done event carries", want, got)
                }
            }
            return@forEachIndexed
        }
        val saved = Json.parseToJsonElement(save().toJson()).jsonObject
        expect["state"]?.let { want ->
            val got = saved["current"]
            if (got != want) {
                throw ScenarioMismatch(n, note, "the current state", want, got)
            }
        }
        // Every active state, a compound or a parallel one with the atomic
        // ones below it. A set: the order the machine lists them in is not
        // part of the answer.
        expect["configuration"]?.let { want ->
            val wanted = idSet(want)
            val got = idSet(saved.getValue("configuration"))
            if (got != wanted) {
                throw ScenarioMismatch(n, note, "the active states", ids(wanted), ids(got))
            }
        }
        expect["variables"]?.jsonObject?.forEach { (name, want) ->
            val got = saved.getValue("variables").jsonObject[name]
            if (!holds(got, want)) {
                throw ScenarioMismatch(n, note, "variable `$name`", want, got)
            }
        }
    }
}

/**
 * Hold step [n] to what it [wants] of one recorder, given what the driver read
 * from it ([got], `null` when the driver records no [recorded]). A step that
 * states nothing of it is not judged on it.
 */
private fun holdRecorded(
    n: Int,
    note: String,
    wants: JsonElement?,
    got: List<JsonElement>?,
    recorded: String,
    what: String,
) {
    if (wants == null) {
        return
    }
    if (got == null) {
        throw ScenarioMismatch(n, note, "this driver records no $recorded, so it cannot hold a step to them")
    }
    val actual = JsonArray(got)
    if (actual != wants) {
        throw ScenarioMismatch(n, note, what, wants, actual)
    }
}

/** The state ids of a JSON array, as a set. */
private fun idSet(ids: JsonElement): Set<String> = ids.jsonArray.map { it.jsonPrimitive.content }.toSet()

/** A set of state ids as a JSON array, in their sorted order, for a message. */
private fun ids(set: Set<String>): JsonArray = JsonArray(set.sorted().map { JsonPrimitive(it) })

/**
 * Whether a saved value is the scenario's. The saved state writes a 64-bit
 * integer as its decimal text, so an expected number matches that text of
 * the same integer. A real is the double its text reads as, however the text
 * spells it (`Double.toString` writes 2^24 as `1.6777216E7`, the scenario as
 * `16777216.0`): parsing is exact, so equal doubles are the same number. A
 * record or a list is held to that field by field and element by element, so
 * a real inside one is judged the same way. Everything else must be equal as
 * written.
 */
internal fun holds(got: JsonElement?, want: JsonElement): Boolean {
    if (got is JsonObject && want is JsonObject) {
        return got.keys == want.keys && want.all { (key, value) -> holds(got[key], value) }
    }
    if (got is JsonArray && want is JsonArray) {
        return got.size == want.size && want.indices.all { holds(got[it], want[it]) }
    }
    if (got is JsonPrimitive && want is JsonPrimitive && got.isString && !want.isString) {
        return want.content.toLongOrNull() != null && got.content == want.content
    }
    if (got is JsonPrimitive && want is JsonPrimitive && !got.isString && !want.isString &&
        (got.content.toLongOrNull() == null || want.content.toLongOrNull() == null)
    ) {
        val gotReal = got.content.toDoubleOrNull()
        val wantReal = want.content.toDoubleOrNull()
        if (gotReal != null && wantReal != null) {
            return gotReal == wantReal
        }
    }
    return got == want
}
