// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 140c4d555915ab51dfdb5b562572972b7994e3bced0046a696f7c65e279b5a12
// generated-at: 1785462747
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test344.scxml:1
package com.sce.w3c

import com.sce.generated.test344.Test344Event
import com.sce.generated.test344.Test344State
import com.sce.generated.test344.Test344StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.9: If a conditional expression cannot be evaluated as a boolean value ('true' or 'false') or if its evaluation causes an error, the SCXML processor MUST place the error 'error.execution' in the internal event queue.
@DisplayName("Test 344 -- W3C SCXML 5.9")
class Test344 : W3CTestBase<Test344State, Test344Event>() {
    override fun createStateMachine() = Test344StateMachine(createEngine())
    override val expectedPassState: Test344State = Test344State.Pass
}
