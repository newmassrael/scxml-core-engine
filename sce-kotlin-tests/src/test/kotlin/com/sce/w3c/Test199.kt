// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: f746fc4eb60c4af2ce8afaa281841d74b58f08c5dc3bf4ba795e1c2351ec0f72
// generated-at: 1780577667
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test199.scxml:1
package com.sce.w3c

import com.sce.generated.test199.Test199Event
import com.sce.generated.test199.Test199State
import com.sce.generated.test199.Test199StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: If the SCXML Processor does not support the type that is specified, it MUST place the event error.execution on the internal event queue.
@DisplayName("Test 199 -- W3C SCXML 6.2")
class Test199 : W3CTestBase<Test199State, Test199Event>() {
    override fun createStateMachine() = Test199StateMachine()
    override val expectedPassState: Test199State = Test199State.Pass
}
