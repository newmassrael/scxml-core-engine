// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: f21fa6fe20b06255f5ff03ff01c6dbc9228fed62e399d58a912b19b086193a03
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test172.scxml:1
package com.sce.w3c

import com.sce.generated.test172.Test172Event
import com.sce.generated.test172.Test172State
import com.sce.generated.test172.Test172StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: If 'eventexpr' is present, the SCXML Processor MUST evaluate it when the parent send element is evaluated and treat the result as if it had been entered as the value of 'event'.
@DisplayName("Test 172 -- W3C SCXML 6.2")
class Test172 : W3CTestBase<Test172State, Test172Event>() {
    override fun createStateMachine() = Test172StateMachine(createEngine())
    override val expectedPassState: Test172State = Test172State.Pass
}
