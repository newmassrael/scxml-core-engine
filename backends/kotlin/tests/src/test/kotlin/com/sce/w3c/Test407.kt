// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test407.scxml:1
package com.sce.w3c

import com.sce.generated.test407.Test407Event
import com.sce.generated.test407.Test407State
import com.sce.generated.test407.Test407StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: To exit a state, the SCXML Processor MUST execute the executable content in the state's onexit handler.
@DisplayName("Test 407 -- W3C SCXML 3.13")
class Test407 : W3CTestBase<Test407State, Test407Event>() {
    override fun createStateMachine() = Test407StateMachine(createEngine())
    override val expectedPassState: Test407State = Test407State.Pass
}
