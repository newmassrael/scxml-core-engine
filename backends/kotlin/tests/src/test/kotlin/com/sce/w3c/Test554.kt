// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2337021aa5cf9b8209b5932f23ab0e04a6899271e435f3620bc1da41d7c4d7b7
// generated-at: 1784381545
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test554.scxml:1
package com.sce.w3c

import com.sce.generated.test554.Test554Event
import com.sce.generated.test554.Test554State
import com.sce.generated.test554.Test554StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: if the evaluation of the invoke element's arguments arguments produces an error, the SCXML Processor MUST terminate the processing of the element without further action.
@DisplayName("Test 554 -- W3C SCXML 6.4")
class Test554 : W3CTestBase<Test554State, Test554Event>() {
    override fun createStateMachine() = Test554StateMachine(createEngine())
    override val expectedPassState: Test554State = Test554State.Pass
    override val timeoutMs: Long = 5000L
}
