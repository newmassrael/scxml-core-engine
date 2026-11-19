// SCE-GENERATED — DO NOT EDIT
// source-hash: e27d05e2af4a00c794d18f0d5665763f23f656fa1a8df31d72d1192ca18fe49d
// source: test242.scxml
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test242.scxml:1
package com.sce.w3c

import com.sce.generated.test242.Test242Event
import com.sce.generated.test242.Test242State
import com.sce.generated.test242.Test242StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: Invoked services MUST also treat values specified by 'src' and content identically.
@DisplayName("Test 242 -- W3C SCXML 6.4")
class Test242 : W3CTestBase<Test242State, Test242Event>() {
    override fun createStateMachine() = Test242StateMachine()
    override val expectedPassState: Test242State = Test242State.Pass
    override val timeoutMs: Long = 5000L
}
