// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: c6c9654e14987bf9fee21998d111ca1385c48c09f2deb9cc862525d124525214
// generated-at: 1785480867
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
