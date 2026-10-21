// SCE-GENERATED — DO NOT EDIT
// source-hash: 75f0d3c43c82c6af748dd1f1aba98de1b9c4f7c51e43e8850cdf3b0490c95be4
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
