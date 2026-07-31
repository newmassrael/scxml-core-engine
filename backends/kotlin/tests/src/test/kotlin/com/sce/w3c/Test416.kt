// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test416.scxml:1
package com.sce.w3c

import com.sce.generated.test416.Test416Event
import com.sce.generated.test416.Test416State
import com.sce.generated.test416.Test416StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: If it [the SCXML processor] has entered a final state that is a child of a compound state [during the last microstep], it MUST generate the event done.state.id, where id is the id of the compound state.
@DisplayName("Test 416 -- W3C SCXML 3.13")
class Test416 : W3CTestBase<Test416State, Test416Event>() {
    override fun createStateMachine() = Test416StateMachine()
    override val expectedPassState: Test416State = Test416State.Pass
}
