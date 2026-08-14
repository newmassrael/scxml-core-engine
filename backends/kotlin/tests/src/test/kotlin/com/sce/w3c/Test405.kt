// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b82119528bc210fbc6e453d658ae079f31e3529ce331b1d6045090bb79eaa2ff
// generated-at: 0
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
