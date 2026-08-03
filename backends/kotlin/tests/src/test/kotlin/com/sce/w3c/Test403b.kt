// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 27e838bbd8708f09c9261661bfb19da674340e525b736fa0c3611ebf1187751e
// generated-at: 0
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
