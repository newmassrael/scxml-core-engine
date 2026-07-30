// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 82d5a5b31a2776e65c97ff666726e5d471238b15131eddc7520023d807e91b34
// generated-at: 1785371281
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test520.scxml:1
package com.sce.w3c

import com.sce.generated.test520.Test520Event
import com.sce.generated.test520.Test520State
import com.sce.generated.test520.Test520StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: If a content child is present, the SCXML Processor MUST use its value as the body of the message.
@DisplayName("Test 520 -- W3C SCXML C.2")
class Test520 : W3CHttpTestBase<Test520State, Test520Event>() {
    override fun createStateMachine() = Test520StateMachine()
    override val expectedPassState: Test520State = Test520State.Pass
}
