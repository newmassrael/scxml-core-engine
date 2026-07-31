// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 1785490018
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test187.scxml:1
package com.sce.w3c

import com.sce.generated.test187.Test187Event
import com.sce.generated.test187.Test187State
import com.sce.generated.test187.Test187StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: If the SCXML session terminates before the delay interval has elapsed, the SCXML Processor MUST discard the message without attempting to deliver it.
@DisplayName("Test 187 -- W3C SCXML 6.2")
class Test187 : W3CTestBase<Test187State, Test187Event>() {
    override fun createStateMachine() = Test187StateMachine()
    override val expectedPassState: Test187State = Test187State.Pass
    override val timeoutMs: Long = 5000L
}
