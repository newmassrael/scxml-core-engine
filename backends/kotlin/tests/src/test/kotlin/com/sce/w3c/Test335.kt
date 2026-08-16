// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: f21fa6fe20b06255f5ff03ff01c6dbc9228fed62e399d58a912b19b086193a03
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test335.scxml:1
package com.sce.w3c

import com.sce.generated.test335.Test335Event
import com.sce.generated.test335.Test335State
import com.sce.generated.test335.Test335StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: If an event was not received from an external entity, the Processor MUST leave the origin field blank.
@DisplayName("Test 335 -- W3C SCXML 5.10")
class Test335 : W3CTestBase<Test335State, Test335Event>() {
    override fun createStateMachine() = Test335StateMachine()
    override val expectedPassState: Test335State = Test335State.Pass
}
