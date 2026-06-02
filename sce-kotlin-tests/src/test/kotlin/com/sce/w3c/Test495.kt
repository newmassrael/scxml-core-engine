// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: beb72c3a9cb76e61aa4916ff585cb6a1d22e66c189bf8cc96c5023dec391d982
// generated-at: 1780379958
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test495.scxml:1
package com.sce.w3c

import com.sce.generated.test495.Test495Event
import com.sce.generated.test495.Test495State
import com.sce.generated.test495.Test495StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: If no errors occur, the receiving Processor MUST convert the message into an SCXML event, using the mapping defined above and insert it into the appropriate queue, as defined in Send Targets.
@DisplayName("Test 495 -- W3C SCXML C.1")
class Test495 : W3CTestBase<Test495State, Test495Event>() {
    override fun createStateMachine() = Test495StateMachine()
    override val expectedPassState: Test495State = Test495State.Pass
}
