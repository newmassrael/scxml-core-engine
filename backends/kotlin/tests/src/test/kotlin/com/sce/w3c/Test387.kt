// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: e136547eba5b1b26d444df3b244f86733d75a97e370ef305f7a135f66e51e2c8
// generated-at: 0
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
