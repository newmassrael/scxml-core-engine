// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test387.scxml:1
package com.sce.w3c

import com.sce.generated.test387.Test387Event
import com.sce.generated.test387.Test387State
import com.sce.generated.test387.Test387StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.10: Before the parent state has been visited for the first time, if a transition is executed that takes the history state as its target, the SCXML processor MUST behave as if the transition had taken the default stored state configuration as its target.
@DisplayName("Test 387 -- W3C SCXML 3.10")
class Test387 : W3CTestBase<Test387State, Test387Event>() {
    override fun createStateMachine() = Test387StateMachine()
    override val expectedPassState: Test387State = Test387State.Pass
}
