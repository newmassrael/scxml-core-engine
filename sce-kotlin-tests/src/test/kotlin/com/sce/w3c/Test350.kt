// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2e32d261d6350eb3a25f2f20128ae90019b36b8835127308d167f05b44688be3
// generated-at: 1779594833
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test350.scxml:1
package com.sce.w3c

import com.sce.generated.test350.Test350Event
import com.sce.generated.test350.Test350State
import com.sce.generated.test350.Test350StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: target'. The sending SCXML Processor MUST take the value of this attribute from the 'target' attribute of the send element. The receiving SCXML Processor MUST use this value to determine which session to deliver the message to.
@DisplayName("Test 350 -- W3C SCXML C.1")
class Test350 : W3CTestBase<Test350State, Test350Event>() {
    override fun createStateMachine() = Test350StateMachine(createEngine())
    override val expectedPassState: Test350State = Test350State.Pass
}
