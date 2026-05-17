// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c1736039ea6628ae1068e428522a9d89bbe2ccef2705503db256c49ec169955e
// generated-at: 1778992486
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test247.scxml:1
package com.sce.w3c

import com.sce.generated.test247.Test247Event
import com.sce.generated.test247.Test247State
import com.sce.generated.test247.Test247StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: If the invoked state machine is of type http://www.w3.org/TR/scxml/ and it reaches a top-level final state, the Processor MUST place the event done.invoke.id on the external event queue of the invoking machine, where id is the invokeid for this invocation
@DisplayName("Test 247 -- W3C SCXML 6.4")
class Test247 : W3CTestBase<Test247State, Test247Event>() {
    override fun createStateMachine() = Test247StateMachine()
    override val expectedPassState: Test247State = Test247State.Pass
}
