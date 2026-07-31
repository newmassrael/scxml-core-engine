// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test250.scxml:1
package com.sce.w3c

import com.sce.generated.test250.Test250Event
import com.sce.generated.test250.Test250State
import com.sce.generated.test250.Test250StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: When an invoked process of type http://www.w3.org/TR/scxml/is cancelled by the invoking process, the Processor MUST execute the onexit handlers for all active states in the invoked session
@DisplayName("Test 250 -- W3C SCXML 6.4")
class Test250 : W3CTestBase<Test250State, Test250Event>() {
    override fun createStateMachine() = Test250StateMachine(createEngine())
    override val expectedPassState: Test250State = Test250State.Final
    override val timeoutMs: Long = 5000L
}
