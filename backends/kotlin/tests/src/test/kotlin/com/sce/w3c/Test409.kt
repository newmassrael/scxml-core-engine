// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 82d5a5b31a2776e65c97ff666726e5d471238b15131eddc7520023d807e91b34
// generated-at: 1785371281
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test409.scxml:1
package com.sce.w3c

import com.sce.generated.test409.Test409Event
import com.sce.generated.test409.Test409State
import com.sce.generated.test409.Test409StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: Finally [after the onexits and canceling the invocations], the Processor MUST remove the state from the active state's list.
@DisplayName("Test 409 -- W3C SCXML 3.13")
class Test409 : W3CTestBase<Test409State, Test409Event>() {
    override fun createStateMachine() = Test409StateMachine()
    override val expectedPassState: Test409State = Test409State.Pass
    override val timeoutMs: Long = 5000L
}
