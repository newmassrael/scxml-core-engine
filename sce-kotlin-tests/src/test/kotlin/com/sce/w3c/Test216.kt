// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: bc7b5b1dd90f65e6c3a4df2e3c4223cf8922d7e6b2d5d124b66683d16074cb6e
// generated-at: 1780362263
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test216.scxml:1
package com.sce.w3c

import com.sce.generated.test216.Test216Event
import com.sce.generated.test216.Test216State
import com.sce.generated.test216.Test216StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: If the srcexpr attribute is present, the SCXML Processor MUST evaluate it when the parent invoke element is evaluated and treat the result as if it had been entered as the value of 'src'.
@DisplayName("Test 216 -- W3C SCXML 6.4")
class Test216 : W3CTestBase<Test216State, Test216Event>() {
    override fun createStateMachine() = Test216StateMachine(createEngine())
    override val expectedPassState: Test216State = Test216State.Pass
    override val timeoutMs: Long = 5000L
}
