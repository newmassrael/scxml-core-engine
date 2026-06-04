// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e03d007af0e666370768a5b0be76775e8be2eb913728a32c0bf7ae79d6929af0
// generated-at: 1780566007
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
