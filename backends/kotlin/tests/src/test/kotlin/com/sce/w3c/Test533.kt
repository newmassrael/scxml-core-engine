// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test533.scxml:1
package com.sce.w3c

import com.sce.generated.test533.Test533Event
import com.sce.generated.test533.Test533State
import com.sce.generated.test533.Test533StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: If a transition has 'type' of "internal", but its source state is not a compound state, its exit set is defined as if it had 'type' of "external".
@DisplayName("Test 533 -- W3C SCXML 3.13")
class Test533 : W3CTestBase<Test533State, Test533Event>() {
    override fun createStateMachine() = Test533StateMachine(createEngine())
    override val expectedPassState: Test533State = Test533State.Pass
}
