// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 8c07c8492e1d3772f35a6f68b5368478c96a8b79da5206f3742243a191d8e3b5
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test421.scxml:1
package com.sce.w3c

import com.sce.generated.test421.Test421Event
import com.sce.generated.test421.Test421State
import com.sce.generated.test421.Test421StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: If the set (of eventless transitions) is empty, the Processor MUST remove events from the internal event queue until the queue is empty or it finds an event that enables a non-empty optimal transition set in the current configuration.
@DisplayName("Test 421 -- W3C SCXML 3.13")
class Test421 : W3CTestBase<Test421State, Test421Event>() {
    override fun createStateMachine() = Test421StateMachine()
    override val expectedPassState: Test421State = Test421State.Pass
}
