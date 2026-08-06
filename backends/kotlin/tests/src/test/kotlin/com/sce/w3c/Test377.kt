// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 670395eefe7272d78e62bf7a7fd9181e96e4a744175a58a4c4de1240c73f57bc
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test377.scxml:1
package com.sce.w3c

import com.sce.generated.test377.Test377Event
import com.sce.generated.test377.Test377State
import com.sce.generated.test377.Test377StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.9: The SCXML processor MUST execute the onexit handlers of a state in document order when the state is exited.
@DisplayName("Test 377 -- W3C SCXML 3.9")
class Test377 : W3CTestBase<Test377State, Test377Event>() {
    override fun createStateMachine() = Test377StateMachine()
    override val expectedPassState: Test377State = Test377State.Pass
}
