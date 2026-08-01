// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 5acba0e3347282f793223e6756c0e705a2e09e70e21550d5eb5dc6ae9d6f33ae
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
