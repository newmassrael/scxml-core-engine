// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 9faef2370910e1d1b12ff0b00a3d63d3578977b6f3f2045b8b014f47fa072349
// generated-at: 1778932425
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
