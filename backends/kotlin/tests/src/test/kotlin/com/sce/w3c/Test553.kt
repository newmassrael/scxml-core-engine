// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test553.scxml:1
package com.sce.w3c

import com.sce.generated.test553.Test553Event
import com.sce.generated.test553.Test553State
import com.sce.generated.test553.Test553StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: If the evaluation of send's arguments produces an error, If the evaluation of send's arguments produces an error, the Processor MUST discard the message without attempting to deliver it.
@DisplayName("Test 553 -- W3C SCXML 6.2")
class Test553 : W3CTestBase<Test553State, Test553Event>() {
    override fun createStateMachine() = Test553StateMachine(createEngine())
    override val expectedPassState: Test553State = Test553State.Pass
    override val timeoutMs: Long = 5000L
}
