// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: f746fc4eb60c4af2ce8afaa281841d74b58f08c5dc3bf4ba795e1c2351ec0f72
// generated-at: 1780577667
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test337.scxml:1
package com.sce.w3c

import com.sce.generated.test337.Test337Event
import com.sce.generated.test337.Test337State
import com.sce.generated.test337.Test337StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: For internal and platform events, the Processor MUST leave the origintype field blank.
@DisplayName("Test 337 -- W3C SCXML 5.10")
class Test337 : W3CTestBase<Test337State, Test337Event>() {
    override fun createStateMachine() = Test337StateMachine()
    override val expectedPassState: Test337State = Test337State.Pass
}
