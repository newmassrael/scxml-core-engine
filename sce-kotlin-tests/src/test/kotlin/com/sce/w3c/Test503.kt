// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: dade9f8de6d0c296ea9dd537c4a48e14404d516e6b96273faf48e4d26f58db4f
// generated-at: 1782564443
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
