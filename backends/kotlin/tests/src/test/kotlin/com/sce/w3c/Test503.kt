// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test503.scxml:1
package com.sce.w3c

import com.sce.generated.test503.Test503Event
import com.sce.generated.test503.Test503State
import com.sce.generated.test503.Test503StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: If the transition does not contain a 'target', its exit set is empty.
@DisplayName("Test 503 -- W3C SCXML 3.13")
class Test503 : W3CTestBase<Test503State, Test503Event>() {
    override fun createStateMachine() = Test503StateMachine(createEngine())
    override val expectedPassState: Test503State = Test503State.Pass
}
