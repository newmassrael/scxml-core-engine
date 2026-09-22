// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 583c0d21905ba9b04748a3d6c74f6cc9796a6884a4c31fd1817c64ee900ca1d2
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test407.scxml:1
package com.sce.w3c

import com.sce.generated.test407.Test407Event
import com.sce.generated.test407.Test407State
import com.sce.generated.test407.Test407StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: To exit a state, the SCXML Processor MUST execute the executable content in the state's onexit handler.
@DisplayName("Test 407 -- W3C SCXML 3.13")
class Test407 : W3CTestBase<Test407State, Test407Event>() {
    override fun createStateMachine() = Test407StateMachine(createEngine())
    override val expectedPassState: Test407State = Test407State.Pass
}
