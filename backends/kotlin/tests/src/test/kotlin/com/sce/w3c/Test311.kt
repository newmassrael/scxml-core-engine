// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 140c4d555915ab51dfdb5b562572972b7994e3bced0046a696f7c65e279b5a12
// generated-at: 1785462747
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test311.scxml:1
package com.sce.w3c

import com.sce.generated.test311.Test311Event
import com.sce.generated.test311.Test311State
import com.sce.generated.test311.Test311StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.9: If a location expression cannot be evaluated to yield a valid location, the SCXML processor MUST place the error error.execution in the internal event queue.
@DisplayName("Test 311 -- W3C SCXML 5.9")
class Test311 : W3CTestBase<Test311State, Test311Event>() {
    override fun createStateMachine() = Test311StateMachine(createEngine())
    override val expectedPassState: Test311State = Test311State.Pass
}
