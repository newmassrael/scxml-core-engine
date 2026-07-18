// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2337021aa5cf9b8209b5932f23ab0e04a6899271e435f3620bc1da41d7c4d7b7
// generated-at: 1784381545
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test496.scxml:1
package com.sce.w3c

import com.sce.generated.test496.Test496Event
import com.sce.generated.test496.Test496State
import com.sce.generated.test496.Test496StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: If the sending SCXML session specifies a session that does not exist or is inaccessible, the SCXML Processor MUST place the error error.communication on the internal event queue of the sending session.
@DisplayName("Test 496 -- W3C SCXML C.1")
class Test496 : W3CTestBase<Test496State, Test496Event>() {
    override fun createStateMachine() = Test496StateMachine(createEngine())
    override val expectedPassState: Test496State = Test496State.Pass
}
