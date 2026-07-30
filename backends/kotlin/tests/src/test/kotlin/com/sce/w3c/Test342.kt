// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 82d5a5b31a2776e65c97ff666726e5d471238b15131eddc7520023d807e91b34
// generated-at: 1785371281
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test342.scxml:1
package com.sce.w3c

import com.sce.generated.test342.Test342Event
import com.sce.generated.test342.Test342State
import com.sce.generated.test342.Test342StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The SCXML Processor MUST set the name field (of the _event variable) to the name of the event.
@DisplayName("Test 342 -- W3C SCXML 5.10")
class Test342 : W3CTestBase<Test342State, Test342Event>() {
    override fun createStateMachine() = Test342StateMachine(createEngine())
    override val expectedPassState: Test342State = Test342State.Pass
}
