// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The Kotlin arm of the `queue` kind's layer 1 (SCE Protocol-Synthesis RFC
// §synth-5-P): every scenario in tests/forge/conformance/queue_contract.json,
// the same ones the Rust, C++ and Go arms run, against the runtime queue its
// storage row names. A row this arm has no runtime for is refused by name,
// never skipped silently.

package com.sce.forge.runtime.queue

import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlin.test.fail
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.int
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.long

/** The element a scenario hands over. */
private class Tracked(val value: Long)

/**
 * What a scenario asks of a queue, so one reading of the steps serves every
 * runtime this arm has. Each runtime reaches its producer and consumer its own
 * way; the scenarios do not know how. A segmented queue has no capacity, and no
 * scenario of one asks for it.
 */
private class Subject(
    private val capacity: Int?,
    private val push: (Tracked) -> PushStatus,
    private val pop: () -> Tracked?,
) {
    fun capacity(): Int = checkNotNull(capacity) { "a segmented queue has no capacity to ask for" }

    fun push(element: Tracked): PushStatus = push.invoke(element)

    fun pop(): Tracked? = pop.invoke()
}

/**
 * Runs [use] on a handle and gives the handle's place back. A scenario holds one
 * handle at a time, so every step takes one and returns it.
 */
private inline fun <H : Any, R> held(handle: H?, release: (H) -> Unit, use: (H) -> R): R {
    val taken = checkNotNull(handle) { "a scenario holds one handle at a time" }
    try {
        return use(taken)
    } finally {
        release(taken)
    }
}

private fun spscSubject(queue: Spsc<Tracked>) = Subject(
    queue.capacity,
    { element -> held(queue.producer(), Spsc.Producer<Tracked>::release) { it.tryPush(element) } },
    { held(queue.consumer(), Spsc.Consumer<Tracked>::release) { it.tryPop() } },
)

private fun scqSubject(queue: Scq<Tracked>) = Subject(
    queue.capacity,
    { element -> held(queue.producer(), Scq.Producer<Tracked>::release) { it.tryPush(element) } },
    { held(queue.consumer(), Scq.Consumer<Tracked>::release) { it.tryPop() } },
)

private fun linkedSubject(queue: LinkedLamport<Tracked>) = Subject(
    null,
    { element -> held(queue.producer(), LinkedLamport.Producer<Tracked>::release) { it.tryPush(element) } },
    { held(queue.consumer(), LinkedLamport.Consumer<Tracked>::release) { it.tryPop() } },
)

private fun lscqSubject(queue: Lscq<Tracked>) = Subject(
    null,
    { element -> held(queue.producer(), Lscq.Producer<Tracked>::release) { it.tryPush(element) } },
    { held(queue.consumer(), Lscq.Consumer<Tracked>::release) { it.tryPop() } },
)

/** The ring size of an SCQ row for a capacity: the capacity rounded up to a power of two. */
internal fun ceilPow2(n: Int): Int {
    var size = 1
    while (size < n) {
        size *= 2
    }
    return size
}

/** Runs one scenario's steps against [subject]; a `destroy` step ends it, since Kotlin has no destructor to run. */
private fun runScenario(subject: Subject, scenario: JsonObject) {
    val id = scenario.getValue("id").jsonPrimitive.content
    for ((index, step) in scenario.getValue("steps").jsonArray.withIndex()) {
        val context = "$id step $index"
        val fields = step.jsonObject
        val op = fields.getValue("op").jsonPrimitive.content
        val expect = fields["expect"]
        when (op) {
            "capacity" -> assertEquals(expect!!.jsonPrimitive.int, subject.capacity(), "$context: capacity")
            "push" -> {
                val value = fields["value"]?.jsonPrimitive?.long ?: fail("$context: a push names its value")
                val want = expect!!.jsonPrimitive.content
                val got = subject.push(Tracked(value))
                when {
                    got == PushStatus.Ok && want == "ok" -> Unit
                    got == PushStatus.Full && want == "full" -> Unit
                    got == PushStatus.OutOfMemory && want == "out_of_memory" -> Unit
                    else -> fail("$context: push gave $got, want $want")
                }
            }
            "pop" -> {
                val popped = subject.pop()
                val want = expect!!.jsonPrimitive
                if (want.isString) {
                    assertEquals("empty", want.content, "$context: a pop expects a number or \"empty\"")
                    assertTrue(popped == null, "$context: expected the queue to be empty, popped ${popped?.value}")
                } else {
                    assertEquals(want.long, popped?.value, "$context: pop")
                }
            }
            // Destruction of held elements is a step for languages with
            // destructors. A Kotlin queue owns no element the collector does not
            // also own; what Kotlin owes instead is that a popped slot is
            // cleared, which QueueRuntimeTest checks directly.
            "destroy" -> return
            else -> fail("$context: unknown op $op")
        }
    }
}

class QueueContractTest {
    private fun contract(): JsonObject {
        val root = System.getProperty("sce.repo.root") ?: fail("sce.repo.root names the checkout the contract is read from")
        val file = File(root, "tests/forge/conformance/queue_contract.json")
        val parsed = Json.parseToJsonElement(file.readText()).jsonObject
        assertEquals(1, parsed.getValue("version").jsonPrimitive.int, "this arm reads version 1 of the contract")
        return parsed
    }

    /**
     * The queue a segmented scenario names, over an allocator that gives the
     * scenario's `segments` at once (all it asks for when it names none).
     * Segmented rows are one producer and one consumer, which select linked
     * Lamport rings, and any other cardinality, which selects a list of SCQ
     * rings (the RFC's selection table).
     */
    private fun segmentedSubject(id: String, scenario: JsonObject): Subject {
        val segment = scenario.getValue("segment").jsonPrimitive.int
        val segments = scenario["segments"]?.jsonPrimitive?.long ?: Long.MAX_VALUE
        val allocator = BudgetAllocator(segments)
        val one = scenario.getValue("producers").jsonPrimitive.content == "one" &&
            scenario.getValue("consumers").jsonPrimitive.content == "one"
        return if (one) {
            linkedSubject(
                LinkedLamport.create<Tracked>(segment, allocator, Progress.LockFree)
                    ?: fail("$id: the allocator refused the first segment"),
            )
        } else {
            lscqSubject(
                Lscq.create<Tracked>(segment, ceilPow2(segment), allocator, Progress.LockFree)
                    ?: fail("$id: the allocator refused the first segment"),
            )
        }
    }

    @Test
    fun everyContractScenarioHolds() {
        val scenarios: JsonArray = contract().getValue("scenarios").jsonArray
        assertTrue(scenarios.isNotEmpty(), "a contract with no scenarios checks nothing")
        for (element in scenarios) {
            val scenario = element.jsonObject
            val id = scenario.getValue("id").jsonPrimitive.content
            val storage = scenario.getValue("storage").jsonPrimitive.content
            val producers = scenario.getValue("producers").jsonPrimitive.content
            val consumers = scenario.getValue("consumers").jsonPrimitive.content
            if (storage == "intrusive") {
                // A collected language has no element to link in place, and the
                // allocation an intrusive list exists to avoid is the collector's.
                // The generator refuses the row for Kotlin by name
                // (queue/storage-runtime-missing), so there is no runtime to run.
                continue
            }
            val subject: Subject = when {
                storage == "segmented" -> segmentedSubject(id, scenario)
                storage == "bounded" && producers == "one" && consumers == "one" ->
                    spscSubject(Spsc(scenario.getValue("capacity").jsonPrimitive.int))
                // Any other cardinality selects the SCQ row (the RFC's
                // selection table), so the three combinations share one
                // runtime.
                storage == "bounded" -> {
                    val capacity = scenario.getValue("capacity").jsonPrimitive.int
                    scqSubject(Scq(capacity, ceilPow2(capacity)))
                }
                else -> fail("$id: the Kotlin arm has no runtime for the row $storage/$producers/$consumers")
            }
            runScenario(subject, scenario)
        }
    }
}
