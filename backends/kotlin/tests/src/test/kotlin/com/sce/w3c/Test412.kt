// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test412.scxml:1
package com.sce.w3c

import com.sce.generated.test412.Test412Event
import com.sce.generated.test412.Test412State
import com.sce.generated.test412.Test412StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: If the state is a default entry state and has an initial child, the SCXML Processor MUST then [after doing the active state add and the onentry handlers] execute the executable content in the initial child's transition.
@DisplayName("Test 412 -- W3C SCXML 3.13")
class Test412 : W3CTestBase<Test412State, Test412Event>() {
    override fun createStateMachine() = Test412StateMachine()
    override val expectedPassState: Test412State = Test412State.Pass
    override val timeoutMs: Long = 5000L
}
