// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 1cfb591080ee0f7028d74f99302d8ee6d7a5b2416447e2ddc2e71e093c1a3c98
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test406.scxml:1
package com.sce.w3c

import com.sce.generated.test406.Test406Event
import com.sce.generated.test406.Test406State
import com.sce.generated.test406.Test406StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: [the SCXML Processor executing a set of transitions] MUST then [after the exits and the transitions] enter the states in the transitions' entry set in entry order.
@DisplayName("Test 406 -- W3C SCXML 3.13")
class Test406 : W3CTestBase<Test406State, Test406Event>() {
    override fun createStateMachine() = Test406StateMachine()
    override val expectedPassState: Test406State = Test406State.Pass
}
