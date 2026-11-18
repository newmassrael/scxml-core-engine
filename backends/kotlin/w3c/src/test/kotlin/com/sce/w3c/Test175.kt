// SCE-GENERATED — DO NOT EDIT
// source-hash: 7e40b95c2d842dcf564705fa3efcf7ff2175df2e7887977f32be9e362c2a3f25
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
