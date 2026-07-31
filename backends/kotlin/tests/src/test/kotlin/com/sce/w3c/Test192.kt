// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test192.scxml:1
package com.sce.w3c

import com.sce.generated.test192.Test192Event
import com.sce.generated.test192.Test192State
import com.sce.generated.test192.Test192StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: [When using the scxml event i/o processor] If the target is the special term '#_invokeid', where invokeid is the invokeid of an SCXML session that the sending session has created by invoke, the Processor MUST must add the event to the external queue of that session.
@DisplayName("Test 192 -- W3C SCXML C.1")
class Test192 : W3CTestBase<Test192State, Test192Event>() {
    override fun createStateMachine() = Test192StateMachine()
    override val expectedPassState: Test192State = Test192State.Pass
    override val timeoutMs: Long = 5000L
}
