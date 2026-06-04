// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: f746fc4eb60c4af2ce8afaa281841d74b58f08c5dc3bf4ba795e1c2351ec0f72
// generated-at: 1780577667
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test336.scxml:1
package com.sce.w3c

import com.sce.generated.test336.Test336Event
import com.sce.generated.test336.Test336State
import com.sce.generated.test336.Test336StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: For external events, the SCXML Processor SHOULD set the origintype field to a value which, in combination with the 'origin' field, will allow the receiver of the event to send a response back to the originating entity.
@DisplayName("Test 336 -- W3C SCXML 5.10")
class Test336 : W3CTestBase<Test336State, Test336Event>() {
    override fun createStateMachine() = Test336StateMachine(createEngine())
    override val expectedPassState: Test336State = Test336State.Pass
}
