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
 * way; the scenarios do not know how.
 */
private interface Subject {
    fun capacity(): Int
    fun push(element: Tracked): PushStatus
    fun pop(): Tracked?
}

private class SpscSubject(private val queue: Spsc<Tracked>) : Subject {
    override fun capacity() = queue.capacity

    override fun push(element: Tracked): PushStatus {
        val producer = checkNotNull(queue.producer()) { "a scenario holds one handle at a time" }
        try {
            return producer.tryPush(element)
        } finally {
            producer.release()
        }
    }

    override fun pop(): Tracked? {
        val consumer = checkNotNull(queue.consumer()) { "a scenario holds one handle at a time" }
        try {
            return consumer.tryPop()
        } finally {
            consumer.release()
        }
    }
}

private class ScqSubject(private val queue: Scq<Tracked>) : Subject {
    override fun capacity() = queue.capacity

    override fun push(element: Tracked): PushStatus {
        val producer = checkNotNull(queue.producer()) { "a scenario holds one handle at a time" }
        try {
            return producer.tryPush(element)
        } finally {
            producer.release()
        }
    }

    override fun pop(): Tracked? {
        val consumer = checkNotNull(queue.consumer()) { "a scenario holds one handle at a time" }
        try {
            return consumer.tryPop()
        } finally {
            consumer.release()
        }
    }
}

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
            if (storage == "segmented") {
                // Not lowered to Kotlin yet; the generator refuses the row by name
                // (queue/storage-runtime-missing), so there is no runtime to run.
                continue
            }
            val capacity = scenario.getValue("capacity").jsonPrimitive.int
            val subject: Subject = when {
                storage == "bounded" && producers == "one" && consumers == "one" -> SpscSubject(Spsc(capacity))
                // Any other cardinality selects the SCQ row (the RFC's
                // selection table), so the three combinations share one
                // runtime.
                storage == "bounded" -> ScqSubject(Scq(capacity, ceilPow2(capacity)))
                else -> fail("$id: the Kotlin arm has no runtime for the row $storage/$producers/$consumers")
            }
            runScenario(subject, scenario)
        }
    }
}
