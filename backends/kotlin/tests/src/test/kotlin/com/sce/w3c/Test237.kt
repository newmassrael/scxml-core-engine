// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2337021aa5cf9b8209b5932f23ab0e04a6899271e435f3620bc1da41d7c4d7b7
// generated-at: 1784381545
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test237.scxml:1
package com.sce.w3c

import com.sce.generated.test237.Test237Event
import com.sce.generated.test237.Test237State
import com.sce.generated.test237.Test237StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: If the invoking session takes a transition out of the state containing the invoke before it receives the 'done.invoke.id' event, the SCXML Processor MUST automatically cancel the invoked component and stop its processing.
@DisplayName("Test 237 -- W3C SCXML 6.4")
class Test237 : W3CTestBase<Test237State, Test237Event>() {
    override fun createStateMachine() = Test237StateMachine()
    override val expectedPassState: Test237State = Test237State.Pass
    override val timeoutMs: Long = 5000L
}
