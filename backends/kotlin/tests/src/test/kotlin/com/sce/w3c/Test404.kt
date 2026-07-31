// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test404.scxml:1
package com.sce.w3c

import com.sce.generated.test404.Test404Event
import com.sce.generated.test404.Test404State
import com.sce.generated.test404.Test404StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: To execute a set of transitions, the SCXML Processor MUST first exit all the states in the transitions' exit set in exit order.
@DisplayName("Test 404 -- W3C SCXML 3.13")
class Test404 : W3CTestBase<Test404State, Test404Event>() {
    override fun createStateMachine() = Test404StateMachine()
    override val expectedPassState: Test404State = Test404State.Pass
}
