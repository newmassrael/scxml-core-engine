// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 583c0d21905ba9b04748a3d6c74f6cc9796a6884a4c31fd1817c64ee900ca1d2
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test375.scxml:1
package com.sce.w3c

import com.sce.generated.test375.Test375Event
import com.sce.generated.test375.Test375State
import com.sce.generated.test375.Test375StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.8: The SCXML processor MUST execute the onentry handlers of a state in document order when the state is entered.
@DisplayName("Test 375 -- W3C SCXML 3.8")
class Test375 : W3CTestBase<Test375State, Test375Event>() {
    override fun createStateMachine() = Test375StateMachine()
    override val expectedPassState: Test375State = Test375State.Pass
}
