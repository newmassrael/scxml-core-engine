// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2c4f76809986b4347703e89a8e901379e8391f815371b53c5a7eecbe187e1cf5
// generated-at: 1781081955
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
