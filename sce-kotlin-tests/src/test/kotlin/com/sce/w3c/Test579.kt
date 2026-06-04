// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 6f9dfe10efef0bb8941aa4cdcfc3ee5783e2349124ce8972e5dc402e99e79f39
// generated-at: 1780582369
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test579.scxml:1
package com.sce.w3c

import com.sce.generated.test579.Test579Event
import com.sce.generated.test579.Test579State
import com.sce.generated.test579.Test579StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.10: Before the parent state has been visited for the first time, if a transition is executed that takes the history state as its target,
@DisplayName("Test 579 -- W3C SCXML 3.10")
class Test579 : W3CTestBase<Test579State, Test579Event>() {
    override fun createStateMachine() = Test579StateMachine(createEngine())
    override val expectedPassState: Test579State = Test579State.Pass
    override val timeoutMs: Long = 5000L
}
