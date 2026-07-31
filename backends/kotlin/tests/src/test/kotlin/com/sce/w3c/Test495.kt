// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
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
