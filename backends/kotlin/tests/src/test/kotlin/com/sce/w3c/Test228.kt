// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test228.scxml:1
package com.sce.w3c

import com.sce.generated.test228.Test228Event
import com.sce.generated.test228.Test228State
import com.sce.generated.test228.Test228StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: the Processor MUST keep track of the unique invokeid and insure that it is included in all events that the invoked service returns to the invoking session.
@DisplayName("Test 228 -- W3C SCXML 6.4")
class Test228 : W3CTestBase<Test228State, Test228Event>() {
    override fun createStateMachine() = Test228StateMachine(createEngine())
    override val expectedPassState: Test228State = Test228State.Pass
    override val timeoutMs: Long = 5000L
}
