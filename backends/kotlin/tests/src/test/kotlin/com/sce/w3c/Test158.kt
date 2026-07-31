// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test158.scxml:1
package com.sce.w3c

import com.sce.generated.test158.Test158Event
import com.sce.generated.test158.Test158State
import com.sce.generated.test158.Test158StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.9: The SCXML processor MUST execute the elements of a block of executable contentin document order.
@DisplayName("Test 158 -- W3C SCXML 4.9")
class Test158 : W3CTestBase<Test158State, Test158Event>() {
    override fun createStateMachine() = Test158StateMachine(createEngine())
    override val expectedPassState: Test158State = Test158State.Pass
}
