// SCE-GENERATED — DO NOT EDIT
// source-hash: 7e40b95c2d842dcf564705fa3efcf7ff2175df2e7887977f32be9e362c2a3f25
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
