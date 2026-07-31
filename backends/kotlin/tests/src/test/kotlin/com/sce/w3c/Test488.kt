// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test488.scxml:1
package com.sce.w3c

import com.sce.generated.test488.Test488Event
import com.sce.generated.test488.Test488State
import com.sce.generated.test488.Test488StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.7: if the evaluation of the 'expr' produces an error, the processor MUST place the error error.execution on the internal event queue.
@DisplayName("Test 488 -- W3C SCXML 5.7")
class Test488 : W3CTestBase<Test488State, Test488Event>() {
    override fun createStateMachine() = Test488StateMachine(createEngine())
    override val expectedPassState: Test488State = Test488State.Pass
}
