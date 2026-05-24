// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2e32d261d6350eb3a25f2f20128ae90019b36b8835127308d167f05b44688be3
// generated-at: 1779589482
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test183.scxml:1
package com.sce.w3c

import com.sce.generated.test183.Test183Event
import com.sce.generated.test183.Test183State
import com.sce.generated.test183.Test183StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: If 'idlocation' is present, the SCXML Processor MUST generate an id when the parent send element is evaluated and store it in this location
@DisplayName("Test 183 -- W3C SCXML 6.2")
class Test183 : W3CTestBase<Test183State, Test183Event>() {
    override fun createStateMachine() = Test183StateMachine(createEngine())
    override val expectedPassState: Test183State = Test183State.Pass
}
