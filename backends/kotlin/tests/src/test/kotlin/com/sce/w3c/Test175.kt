// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 0afb1f0f0230f40c373aa80a890f61f2cc90b35724e7d86493a9e44e197b2d1b
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test175.scxml:1
package com.sce.w3c

import com.sce.generated.test175.Test175Event
import com.sce.generated.test175.Test175State
import com.sce.generated.test175.Test175StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: If 'delayexpr' is present, the SCXML Processor MUST evaluate it when the parent send element is evaluated and treat the result as if it had been entered as the value of 'delay'.
@DisplayName("Test 175 -- W3C SCXML 6.2")
class Test175 : W3CTestBase<Test175State, Test175Event>() {
    override fun createStateMachine() = Test175StateMachine(createEngine())
    override val expectedPassState: Test175State = Test175State.Pass
    override val timeoutMs: Long = 5000L
}
