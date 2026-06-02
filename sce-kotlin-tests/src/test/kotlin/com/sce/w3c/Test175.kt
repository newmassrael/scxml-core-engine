// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e8782a5c8351481fc8f6e7fcdb09caae80cbe9e47c6019dcf15afff703e3c3b3
// generated-at: 1780407549
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
