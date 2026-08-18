// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b282d63ae523573aa0c92c912a0dda6cb9508b9193d3508ff15b98a4ec52a48a
// generated-at: 0
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
