// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c1736039ea6628ae1068e428522a9d89bbe2ccef2705503db256c49ec169955e
// generated-at: 1778992486
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test529.scxml:1
package com.sce.w3c

import com.sce.generated.test529.Test529Event
import com.sce.generated.test529.Test529State
import com.sce.generated.test529.Test529StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.6: If the 'expr' attribute is not present, the Processor MUST use the children of content as the output.
@DisplayName("Test 529 -- W3C SCXML 5.6")
class Test529 : W3CTestBase<Test529State, Test529Event>() {
    override fun createStateMachine() = Test529StateMachine(createEngine())
    override val expectedPassState: Test529State = Test529State.Pass
}
