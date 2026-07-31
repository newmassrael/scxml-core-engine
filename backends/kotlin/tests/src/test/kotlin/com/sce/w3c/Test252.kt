// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 1785490018
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test252.scxml:1
package com.sce.w3c

import com.sce.generated.test252.Test252Event
import com.sce.generated.test252.Test252State
import com.sce.generated.test252.Test252StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: Once it cancels an invoked session, the Processor MUST NOT insert any events it receives from the invoked session into the external event queue of the invoking session.
@DisplayName("Test 252 -- W3C SCXML 6.4")
class Test252 : W3CTestBase<Test252State, Test252Event>() {
    override fun createStateMachine() = Test252StateMachine()
    override val expectedPassState: Test252State = Test252State.Pass
    override val timeoutMs: Long = 5000L
}
