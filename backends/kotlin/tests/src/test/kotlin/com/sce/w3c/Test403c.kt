// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: f8935a2b1ceca80a03ff3489cc9f8dcbccd8c2b85fc58c3b848403d6a2672153
// generated-at: 0
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
