// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 6f9dfe10efef0bb8941aa4cdcfc3ee5783e2349124ce8972e5dc402e99e79f39
// generated-at: 1780582369
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
