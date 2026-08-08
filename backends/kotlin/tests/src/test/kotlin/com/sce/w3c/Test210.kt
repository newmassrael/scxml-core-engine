// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 87472be6c3c3f32bb0fc76df8bdb613c176bab0c4f79720c0934dff4542328b8
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test210.scxml:1
package com.sce.w3c

import com.sce.generated.test210.Test210Event
import com.sce.generated.test210.Test210State
import com.sce.generated.test210.Test210StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.3: If the 'sendidexpr' attribute is present, the SCXML Processor MUST evaluate it when the parent cancel element is evaluated and treat the result as if it had been entered as the value of 'sendid'.
@DisplayName("Test 210 -- W3C SCXML 6.3")
class Test210 : W3CTestBase<Test210State, Test210Event>() {
    override fun createStateMachine() = Test210StateMachine(createEngine())
    override val expectedPassState: Test210State = Test210State.Pass
    override val timeoutMs: Long = 5000L
}
