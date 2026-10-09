// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The queue kind's generated files (SCE Protocol-Synthesis RFC §synth-5-P),
// compiled and used.
//
// sce-build's tests read what the generator writes for a queue and look for
// text in it, which shows the text is what the template says and nothing about
// whether the names in it are the names the runtime has. Here the file the
// generator writes for each bounded algorithm row is compiled into this test
// over the element class it imports, and the queue it defines is used: a rename
// in the runtime, or a constant the template computes wrongly, stops the build
// or fails an assertion. The runtime's own properties are QueueRuntimeTest's;
// this file holds what the generated file adds.
//
// The files come from tests/forge/resources/queue_conformance_*.scxml, written
// by the generateQueueFixtures task of build.gradle.kts.

package com.sce.forge.runtime.queue

import com.sce.generated.queue_conformance_event.QueueConformanceEvent
import com.sce.generated.queue_conformance_scq.ALGORITHM as SCQ_ALGORITHM
import com.sce.generated.queue_conformance_scq.CAPACITY as SCQ_CAPACITY
import com.sce.generated.queue_conformance_scq.DECLARED_PROGRESS as SCQ_DECLARED_PROGRESS
import com.sce.generated.queue_conformance_scq.PARTICIPANTS
import com.sce.generated.queue_conformance_scq.POP_PROGRESS as SCQ_POP_PROGRESS
import com.sce.generated.queue_conformance_scq.PUSH_PROGRESS as SCQ_PUSH_PROGRESS
import com.sce.generated.queue_conformance_scq.QueueConformanceScq
import com.sce.generated.queue_conformance_scq.RING_SLOTS
import com.sce.generated.queue_conformance_scq.WRAP_BOUND_OPS as SCQ_WRAP_BOUND_OPS
import com.sce.generated.queue_conformance_scq.newQueueConformanceScq
import com.sce.generated.queue_conformance_spsc.ALGORITHM as SPSC_ALGORITHM
import com.sce.generated.queue_conformance_spsc.CAPACITY as SPSC_CAPACITY
import com.sce.generated.queue_conformance_spsc.DECLARED_PROGRESS as SPSC_DECLARED_PROGRESS
import com.sce.generated.queue_conformance_spsc.POP_PROGRESS as SPSC_POP_PROGRESS
import com.sce.generated.queue_conformance_spsc.PUSH_PROGRESS as SPSC_PUSH_PROGRESS
import com.sce.generated.queue_conformance_spsc.QueueConformanceSpsc
import com.sce.generated.queue_conformance_spsc.newQueueConformanceSpsc
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

// The generated alias is the runtime's queue over the element class: these
// accept the alias and hand back the runtime's type, and do not compile
// otherwise.
private fun runtimeQueue(queue: QueueConformanceSpsc): Spsc<QueueConformanceEvent> = queue

private fun runtimeQueue(queue: QueueConformanceScq): Scq<QueueConformanceEvent> = queue

private fun event(sensor: Int, value: Int) = QueueConformanceEvent(sensor_id = sensor.toUByte(), value = value.toUShort())

class QueueGeneratedTest {
    @Test
    fun theLamportRingFileStatesWhatTheDocumentRequiredAndWhatItGives() {
        assertEquals(4, SPSC_CAPACITY, "the capacity is the document's: four")
        for (progress in listOf(SPSC_DECLARED_PROGRESS, SPSC_PUSH_PROGRESS, SPSC_POP_PROGRESS)) {
            assertEquals("wait-free", progress)
        }
        assertTrue(SPSC_ALGORITHM.contains("Lamport"), "one producer and one consumer select the Lamport ring: $SPSC_ALGORITHM")
        assertEquals(SPSC_CAPACITY, runtimeQueue(newQueueConformanceSpsc()).capacity)
    }

    @Test
    fun theLamportRingFileHandsElementsOverInOrderAndRefusesWhenFull() {
        val queue = newQueueConformanceSpsc()
        val producer = assertNotNull(queue.producer())
        val consumer = assertNotNull(queue.consumer())

        assertNull(consumer.tryPop(), "a new queue is empty")
        for (i in 0 until SPSC_CAPACITY) {
            assertEquals(PushStatus.Ok, producer.tryPush(event(1, i)), "push $i of $SPSC_CAPACITY must fit")
        }
        assertEquals(PushStatus.Full, producer.tryPush(event(9, 99)), "a full queue refuses the push")
        for (i in 0 until SPSC_CAPACITY) {
            val got = assertNotNull(consumer.tryPop(), "pop $i")
            assertEquals(1.toUByte(), got.sensor_id)
            assertEquals(i.toUShort(), got.value, "pop $i: first in, first out")
        }
        assertNull(consumer.tryPop(), "drained")
    }

    @Test
    fun theScqFileStatesTheRingItNeedsAndWhatItGives() {
        assertEquals(6, SCQ_CAPACITY, "the capacity is the document's: six")
        assertEquals(3, PARTICIPANTS, "the participants are the document's: three")
        assertEquals(8, RING_SLOTS, "the next power of two at or above both numbers")
        assertEquals(WRAP_BOUND_OPS, SCQ_WRAP_BOUND_OPS, "the bound is the runtime's")
        assertEquals("lock-free", SCQ_DECLARED_PROGRESS)
        assertEquals("lock-free", SCQ_PUSH_PROGRESS)
        assertEquals("lock-free", SCQ_POP_PROGRESS)
        assertTrue(SCQ_ALGORITHM.contains("SCQ"), "many producers and consumers select SCQ: $SCQ_ALGORITHM")
        val queue = runtimeQueue(newQueueConformanceScq())
        assertEquals(SCQ_CAPACITY, queue.capacity)
        assertEquals(RING_SLOTS, queue.ringSlots)
    }

    @Test
    fun theScqFileFitsExactlyItsCapacityWhenNothingElseRuns() {
        val queue = newQueueConformanceScq()
        val producer = assertNotNull(queue.producer())
        val consumer = assertNotNull(queue.consumer())
        for (i in 0 until SCQ_CAPACITY) {
            assertEquals(PushStatus.Ok, producer.tryPush(event(2, i)), "push $i of $SCQ_CAPACITY must fit")
        }
        assertEquals(PushStatus.Full, producer.tryPush(event(9, 99)), "a full queue refuses the push")
        for (i in 0 until SCQ_CAPACITY) {
            val got = assertNotNull(consumer.tryPop(), "pop $i")
            assertEquals(i.toUShort(), got.value, "pop $i: first in, first out")
        }
        assertNull(consumer.tryPop(), "drained")
    }
}
