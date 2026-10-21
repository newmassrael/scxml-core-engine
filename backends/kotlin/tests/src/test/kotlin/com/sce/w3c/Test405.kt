// SCE-GENERATED — DO NOT EDIT
// source-hash: 75f0d3c43c82c6af748dd1f1aba98de1b9c4f7c51e43e8850cdf3b0490c95be4
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test405.scxml:1
package com.sce.w3c

import com.sce.generated.test405.Test405Event
import com.sce.generated.test405.Test405State
import com.sce.generated.test405.Test405StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: [the SCXML Processor executing a set of transitions] MUST then [after the onexits] execute the executable content contained in the transitions in document order.
@DisplayName("Test 405 -- W3C SCXML 3.13")
class Test405 : W3CTestBase<Test405State, Test405Event>() {
    override fun createStateMachine() = Test405StateMachine()
    override val expectedPassState: Test405State = Test405State.Pass
}
