// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test321.scxml:1
package com.sce.w3c

import com.sce.generated.test321.Test321Event
import com.sce.generated.test321.Test321State
import com.sce.generated.test321.Test321StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST bind the variable _sessionid at load time to the system-generated id for the current SCXML session.
@DisplayName("Test 321 -- W3C SCXML 5.10")
class Test321 : W3CTestBase<Test321State, Test321Event>() {
    override fun createStateMachine() = Test321StateMachine(createEngine())
    override val expectedPassState: Test321State = Test321State.Pass
}
