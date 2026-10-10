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
import com.sce.generated.queue_conformance_scq.SCQ_ARCHITECTURES
import com.sce.generated.queue_conformance_scq.requireLockFree64BitAtomics
import com.sce.generated.queue_conformance_scq.WRAP_BOUND_OPS as SCQ_WRAP_BOUND_OPS
import com.sce.generated.queue_conformance_scq.newQueueConformanceScq
import com.sce.generated.queue_conformance_segmented.ALGORITHM as LINKED_ALGORITHM
import com.sce.generated.queue_conformance_segmented.ALLOCATOR_PROGRESS as LINKED_ALLOCATOR_PROGRESS
import com.sce.generated.queue_conformance_segmented.ALLOCATOR_PROGRESS_WORD as LINKED_ALLOCATOR_PROGRESS_WORD
import com.sce.generated.queue_conformance_segmented.DECLARED_PROGRESS as LINKED_DECLARED_PROGRESS
import com.sce.generated.queue_conformance_segmented.POP_PROGRESS as LINKED_POP_PROGRESS
import com.sce.generated.queue_conformance_segmented.PUSH_PROGRESS as LINKED_PUSH_PROGRESS
import com.sce.generated.queue_conformance_segmented.QueueConformanceSegmented
import com.sce.generated.queue_conformance_segmented.SEGMENT as LINKED_SEGMENT
import com.sce.generated.queue_conformance_segmented.newQueueConformanceSegmented
import com.sce.generated.queue_conformance_segmented_many.ALGORITHM as LSCQ_ALGORITHM
import com.sce.generated.queue_conformance_segmented_many.ALLOCATOR_PROGRESS as LSCQ_ALLOCATOR_PROGRESS
import com.sce.generated.queue_conformance_segmented_many.DECLARED_PROGRESS as LSCQ_DECLARED_PROGRESS
import com.sce.generated.queue_conformance_segmented_many.PARTICIPANTS as LSCQ_PARTICIPANTS
import com.sce.generated.queue_conformance_segmented_many.POP_PROGRESS as LSCQ_POP_PROGRESS
import com.sce.generated.queue_conformance_segmented_many.PUSH_PROGRESS as LSCQ_PUSH_PROGRESS
import com.sce.generated.queue_conformance_segmented_many.QueueConformanceSegmentedMany
import com.sce.generated.queue_conformance_segmented_many.RING_SLOTS as LSCQ_RING_SLOTS
import com.sce.generated.queue_conformance_segmented_many.SEGMENT as LSCQ_SEGMENT
import com.sce.generated.queue_conformance_segmented_many.newQueueConformanceSegmentedMany
import com.sce.generated.queue_conformance_spsc.ALGORITHM as SPSC_ALGORITHM
import com.sce.generated.queue_conformance_spsc.CAPACITY as SPSC_CAPACITY
import com.sce.generated.queue_conformance_spsc.DECLARED_PROGRESS as SPSC_DECLARED_PROGRESS
import com.sce.generated.queue_conformance_spsc.POP_PROGRESS as SPSC_POP_PROGRESS
import com.sce.generated.queue_conformance_spsc.PUSH_PROGRESS as SPSC_PUSH_PROGRESS
import com.sce.generated.queue_conformance_spsc.QueueConformanceSpsc
import com.sce.generated.queue_conformance_spsc.newQueueConformanceSpsc
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

// The generated alias is the runtime's queue over the element class: these
// accept the alias and hand back the runtime's type, and do not compile
// otherwise.
private fun runtimeQueue(queue: QueueConformanceSpsc): Spsc<QueueConformanceEvent> = queue

private fun runtimeQueue(queue: QueueConformanceScq): Scq<QueueConformanceEvent> = queue

private fun runtimeQueue(queue: QueueConformanceSegmented): LinkedLamport<QueueConformanceEvent> = queue

private fun runtimeQueue(queue: QueueConformanceSegmentedMany): Lscq<QueueConformanceEvent> = queue

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
    fun theScqFileFailsConstructionOnAProcessorWithNoLockFree64BitAtomics() {
        // The running JVM is one the list names, or this test run could not
        // build the queue its other tests use.
        requireLockFree64BitAtomics()
        for (arch in listOf("amd64", "aarch64", "riscv64")) {
            assertTrue(arch in SCQ_ARCHITECTURES, arch)
            requireLockFree64BitAtomics(arch)
        }
        for (arch in listOf("x86", "arm", "mips", "ppc", "")) {
            val failure = assertFailsWith<IllegalStateException>("os.arch '$arch'") {
                requireLockFree64BitAtomics(arch)
            }
            assertTrue(failure.message!!.contains("lock-free 64-bit atomics"), failure.message)
            assertTrue(failure.message!!.contains("'$arch'"), "the message names the architecture: ${failure.message}")
        }
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

    @Test
    fun theLinkedLamportFileStatesWhatTheDocumentRequiredAndWhatItGives() {
        assertEquals(3, LINKED_SEGMENT, "the segment is the document's: three")
        assertEquals("lock-free", LINKED_DECLARED_PROGRESS)
        assertEquals("lock-free", LINKED_PUSH_PROGRESS, "a push no stronger than its allocator")
        assertEquals("wait-free", LINKED_POP_PROGRESS)
        assertTrue(LINKED_ALGORITHM.contains("Lamport"), "one and one select linked Lamport rings: $LINKED_ALGORITHM")
        assertEquals(Progress.LockFree, LINKED_ALLOCATOR_PROGRESS, "the allocator progress is the document's")
        assertEquals("lock-free", LINKED_ALLOCATOR_PROGRESS_WORD)
        val queue = assertNotNull(newQueueConformanceSegmented(BudgetAllocator()))
        assertEquals(LINKED_SEGMENT, runtimeQueue(queue).segment)
    }

    @Test
    fun theLinkedLamportFileHandsElementsOverAcrossSegmentsAndReportsARefusedSegment() {
        // Two segments of three: six elements, then the allocator refuses.
        val allocator = BudgetAllocator(limit = 2)
        val queue = assertNotNull(newQueueConformanceSegmented(allocator))
        val producer = assertNotNull(queue.producer())
        val consumer = assertNotNull(queue.consumer())

        assertNull(consumer.tryPop(), "a new queue is empty")
        for (i in 0 until 2 * LINKED_SEGMENT) {
            assertEquals(PushStatus.Ok, producer.tryPush(event(1, i)), "push $i fits the two segments")
        }
        assertEquals(PushStatus.OutOfMemory, producer.tryPush(event(9, 99)), "the allocator refuses a third segment")
        for (i in 0 until 2 * LINKED_SEGMENT) {
            val got = assertNotNull(consumer.tryPop(), "pop $i")
            assertEquals(i.toUShort(), got.value, "pop $i: first in, first out")
        }
        assertNull(consumer.tryPop(), "drained")
        assertEquals(PushStatus.Ok, producer.tryPush(event(1, 100)), "a segment the consumer gave back is room again")
    }

    @Test
    fun theLinkedLamportFileRefusesAnAllocatorThatGivesLessThanTheDocumentDeclared() {
        val failure = assertFailsWith<IllegalArgumentException> {
            newQueueConformanceSegmented(BudgetAllocator(progress = Progress.Blocking))
        }
        assertTrue(failure.message!!.contains("Blocking"), "the message names what the allocator gives: ${failure.message}")
        assertNull(newQueueConformanceSegmented(BudgetAllocator(limit = 0)), "an allocator with no segment to give")
    }

    @Test
    fun theLscqFileStatesTheRingItNeedsAndWhatItGives() {
        assertEquals(3, LSCQ_SEGMENT, "the segment is the document's: three")
        assertEquals(2, LSCQ_PARTICIPANTS, "the participants are the document's: two")
        assertEquals(4, LSCQ_RING_SLOTS, "the next power of two at or above max(segment, participants)")
        assertEquals("lock-free", LSCQ_DECLARED_PROGRESS)
        assertEquals("lock-free", LSCQ_PUSH_PROGRESS)
        assertEquals("lock-free", LSCQ_POP_PROGRESS)
        assertTrue(LSCQ_ALGORITHM.contains("LSCQ"), "many and many select LSCQ: $LSCQ_ALGORITHM")
        assertEquals(Progress.LockFree, LSCQ_ALLOCATOR_PROGRESS)
        val queue = assertNotNull(newQueueConformanceSegmentedMany(BudgetAllocator()))
        assertEquals(LSCQ_SEGMENT, runtimeQueue(queue).segment)
        assertEquals(LSCQ_RING_SLOTS, runtimeQueue(queue).ringSlots)
    }

    @Test
    fun theLscqFileHandsElementsOverAcrossSegmentsAndGivesEachSegmentBack() {
        val allocator = BudgetAllocator()
        val queue = assertNotNull(newQueueConformanceSegmentedMany(allocator))
        val producer = assertNotNull(queue.producer())
        val consumer = assertNotNull(queue.consumer())
        val total = 4 * LSCQ_SEGMENT + 1

        assertNull(consumer.tryPop(), "a new queue is empty")
        for (i in 0 until total) {
            assertEquals(PushStatus.Ok, producer.tryPush(event(2, i)), "push $i")
        }
        assertTrue(allocator.granted >= 5, "thirteen elements of three need at least five segments: ${allocator.granted}")
        for (i in 0 until total) {
            val got = assertNotNull(consumer.tryPop(), "pop $i")
            assertEquals(i.toUShort(), got.value, "pop $i: first in, first out")
        }
        assertNull(consumer.tryPop(), "drained")
        assertEquals(1L, allocator.live, "only the newest segment is still the queue's")
    }

    @Test
    fun theLscqFileRefusesAnAllocatorThatGivesLessThanTheDocumentDeclared() {
        assertFailsWith<IllegalArgumentException> {
            newQueueConformanceSegmentedMany(BudgetAllocator(progress = Progress.Blocking))
        }
        assertNull(newQueueConformanceSegmentedMany(BudgetAllocator(limit = 0)), "an allocator with no segment to give")
    }
}
