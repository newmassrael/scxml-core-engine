// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test253.scxml:1
package com.sce.w3c

import com.sce.generated.test253.Test253Event
import com.sce.generated.test253.Test253State
import com.sce.generated.test253.Test253StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: When the invoked session is of type http://www.w3.org/TR/scxml/, The SCXML Processor MUST support the use of SCXML Event/IO processor (E.1 SCXML Event I/O Processor) to communicate between the invoking and the invoked sessions.
@DisplayName("Test 253 -- W3C SCXML 6.4")
class Test253 : W3CTestBase<Test253State, Test253Event>() {
    override fun createStateMachine() = Test253StateMachine(createEngine())
    override val expectedPassState: Test253State = Test253State.Pass
}
