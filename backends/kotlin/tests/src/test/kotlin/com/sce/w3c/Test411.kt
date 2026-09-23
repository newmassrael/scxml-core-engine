// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 8c07c8492e1d3772f35a6f68b5368478c96a8b79da5206f3742243a191d8e3b5
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test411.scxml:1
package com.sce.w3c

import com.sce.generated.test411.Test411Event
import com.sce.generated.test411.Test411State
import com.sce.generated.test411.Test411StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: To enter a state, the SCXML Processor MUST add the state to the active state's list. Then it MUST execute the executable content in the state's onentry handler.
@DisplayName("Test 411 -- W3C SCXML 3.13")
class Test411 : W3CTestBase<Test411State, Test411Event>() {
    override fun createStateMachine() = Test411StateMachine()
    override val expectedPassState: Test411State = Test411State.Pass
    override val timeoutMs: Long = 5000L
}
