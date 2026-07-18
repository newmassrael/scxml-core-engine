// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2337021aa5cf9b8209b5932f23ab0e04a6899271e435f3620bc1da41d7c4d7b7
// generated-at: 1784381545
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test403c.scxml:1
package com.sce.w3c

import com.sce.generated.test403c.Test403cEvent
import com.sce.generated.test403c.Test403cState
import com.sce.generated.test403c.Test403cStateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: To execute a microstep, the SCXML Processor MUST execute the transitions in the corresponding optimal enabled transition set, where the optimal transition set enabled by event E in state configuration C is the largest set of transitions such that a) each transition in the set is optimally enabled by E in an atomic state in C b) no transition conflicts with another transition in the set c) there is no optimally enabled transition outside
@DisplayName("Test 403c -- W3C SCXML 3.13")
class Test403c : W3CTestBase<Test403cState, Test403cEvent>() {
    override fun createStateMachine() = Test403cStateMachine(createEngine())
    override val expectedPassState: Test403cState = Test403cState.Pass
}
