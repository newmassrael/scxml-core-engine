// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test247.scxml:1
package com.sce.w3c

import com.sce.generated.test247.Test247Event
import com.sce.generated.test247.Test247State
import com.sce.generated.test247.Test247StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: If the invoked state machine is of type http://www.w3.org/TR/scxml/ and it reaches a top-level final state, the Processor MUST place the event done.invoke.id on the external event queue of the invoking machine, where id is the invokeid for this invocation
@DisplayName("Test 247 -- W3C SCXML 6.4")
class Test247 : W3CTestBase<Test247State, Test247Event>() {
    override fun createStateMachine() = Test247StateMachine()
    override val expectedPassState: Test247State = Test247State.Pass
}
