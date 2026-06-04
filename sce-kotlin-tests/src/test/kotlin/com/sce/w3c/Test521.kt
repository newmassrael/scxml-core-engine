// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: f746fc4eb60c4af2ce8afaa281841d74b58f08c5dc3bf4ba795e1c2351ec0f72
// generated-at: 1780577667
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test521.scxml:1
package com.sce.w3c

import com.sce.generated.test521.Test521Event
import com.sce.generated.test521.Test521State
import com.sce.generated.test521.Test521StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: f the Processor cannot dispatch the event, it MUST place the error error.communication on the internal event queue of the session that attempted to send the event.
@DisplayName("Test 521 -- W3C SCXML 6.2")
class Test521 : W3CTestBase<Test521State, Test521Event>() {
    override fun createStateMachine() = Test521StateMachine(createEngine())
    override val expectedPassState: Test521State = Test521State.Pass
}
