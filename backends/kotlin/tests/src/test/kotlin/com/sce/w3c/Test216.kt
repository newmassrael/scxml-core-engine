// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 82d5a5b31a2776e65c97ff666726e5d471238b15131eddc7520023d807e91b34
// generated-at: 1785371281
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
