// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 82d5a5b31a2776e65c97ff666726e5d471238b15131eddc7520023d807e91b34
// generated-at: 1785371281
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
