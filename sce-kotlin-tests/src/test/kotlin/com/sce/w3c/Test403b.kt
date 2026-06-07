// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 66bc1c3694f90e60100c842d2a53cd8c05682260c1809ba387d157940d7d6e1d
// generated-at: 1780836426
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test403b.scxml:1
package com.sce.w3c

import com.sce.generated.test403b.Test403bEvent
import com.sce.generated.test403b.Test403bState
import com.sce.generated.test403b.Test403bStateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: To execute a microstep, the SCXML Processor MUST execute the transitions in the corresponding optimal enabled transition set, where the optimal transition set enabled by event E in state configuration C is the largest set of transitions such that a) each transition in the set is optimally enabled by E in an atomic state in C b) no transition conflicts with another transition in the set c) there is no optimally enabled transition outside
@DisplayName("Test 403b -- W3C SCXML 3.13")
class Test403b : W3CTestBase<Test403bState, Test403bEvent>() {
    override fun createStateMachine() = Test403bStateMachine(createEngine())
    override val expectedPassState: Test403bState = Test403bState.Pass
}
