// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 025e57d78939dcd3c3bbc54b606a62c00b45f367a9a3d9faa2cdd4bf5896d8fc
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
