// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test191.scxml:1
package com.sce.w3c

import com.sce.generated.test191.Test191Event
import com.sce.generated.test191.Test191State
import com.sce.generated.test191.Test191StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: [When using the scxml event i/o processor] If the target is the special term '#_parent', the Processor MUST add the event to the external event queue of the SCXML session that invoked the sending session, if there is one.
@DisplayName("Test 191 -- W3C SCXML C.1")
class Test191 : W3CTestBase<Test191State, Test191Event>() {
    override fun createStateMachine() = Test191StateMachine()
    override val expectedPassState: Test191State = Test191State.Pass
    override val timeoutMs: Long = 5000L
}
