// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 0
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
